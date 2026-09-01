package server

import (
	"context"
	"fmt"
	"io"
	"log"
	"net"
	"strings"
	"time"

	"golang.org/x/crypto/ssh"

	"tunnl.gg/internal/config"
	"tunnl.gg/internal/subdomain"
	"tunnl.gg/internal/tunnel"
)

type tcpipForwardRequest struct {
	BindAddr string
	BindPort uint32
}

type forwardedTCPPayload struct {
	Addr       string
	Port       uint32
	OriginAddr string
	OriginPort uint32
}

// HandleSSHConnection handles a new SSH connection
// execRequest is the payload of an SSH "exec" request. Clients pass the
// subdomain they want as the command:
//
//	ssh -t -R 80:localhost:8080 <domain> myapp
type execRequest struct {
	Command string
}

// HandleSSHConnection handles a new SSH connection
func (s *Server) HandleSSHConnection(conn net.Conn) {
	clientIP := "unknown"
	if tcpConn, ok := conn.(*net.TCPConn); ok {
		if tcpAddr, ok := tcpConn.RemoteAddr().(*net.TCPAddr); ok {
			clientIP = tcpAddr.IP.String()
		}
		// Set TCP_NODELAY to prevent SSH library from logging errors
		tcpConn.SetNoDelay(true)
	}

	conn.SetDeadline(time.Now().Add(config.SSHHandshakeTimeout))
	sshConn, chans, reqs, err := ssh.NewServerConn(conn, s.sshConfig)
	if err != nil {
		log.Printf("SSH handshake failed: %v", err)
		return
	}
	conn.SetDeadline(time.Time{}) // clear deadline after successful handshake
	defer sshConn.Close()

	s.IncrementConnections()

	tunnelListener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		log.Printf("Failed to create tunnel listener: %v", err)
		return
	}
	// Ensure listener is closed on early return (before the tunnel is claimed).
	// This is safe even after claiming since net.Listener.Close() is idempotent.
	defer tunnelListener.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	var bindAddr string
	var bindPort uint32
	forwardRequested := make(chan struct{})

	// Handle global requests (port forwarding)
	go func() {
		requested := false
		for {
			select {
			case req, ok := <-reqs:
				if !ok {
					return
				}
				switch req.Type {
				case "tcpip-forward":
					if requested {
						req.Reply(false, nil)
						continue
					}
					var fwdReq tcpipForwardRequest
					if err := ssh.Unmarshal(req.Payload, &fwdReq); err != nil {
						req.Reply(false, nil)
						continue
					}
					bindAddr = fwdReq.BindAddr
					bindPort = fwdReq.BindPort
					requested = true
					close(forwardRequested)
					req.Reply(true, nil)
				case "cancel-tcpip-forward":
					req.Reply(true, nil)
				default:
					req.Reply(false, nil)
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	select {
	case <-forwardRequested:
	case <-time.After(30 * time.Second):
		log.Printf("Timeout waiting for tcpip-forward request from %s", sshConn.RemoteAddr())
		return
	}

	// Wait for a session channel with timeout
	sessionReceived := make(chan ssh.NewChannel, 1)
	go func() {
		for {
			select {
			case newChannel, ok := <-chans:
				if !ok {
					return
				}
				if newChannel.ChannelType() == "session" {
					sessionReceived <- newChannel
					return
				}
				newChannel.Reject(ssh.UnknownChannelType, "unknown channel type")
			case <-ctx.Done():
				return
			}
		}
	}()

	var sessionChannel ssh.NewChannel
	select {
	case sessionChannel = <-sessionReceived:
	case <-time.After(5 * time.Second):
		log.Printf("Connection from %s rejected: no session channel (use ssh -t)", sshConn.RemoteAddr())
		return
	}

	channel, requests, err := sessionChannel.Accept()
	if err != nil {
		log.Printf("Failed to accept session channel: %v", err)
		return
	}

	// The subdomain, if the client asked for one, arrives as the SSH command.
	// Claiming has to wait for it, so nothing is registered until now.
	requested, ok := awaitCommand(requests)
	if !ok {
		log.Printf("Connection from %s rejected: no exec or shell request", sshConn.RemoteAddr())
		return
	}

	var sub string
	var tun *tunnel.Tunnel
	if requested == "" {
		sub, tun, err = s.ClaimGenerated(tunnelListener, bindAddr, bindPort, clientIP)
	} else if !subdomain.IsValid(requested) {
		err = fmt.Errorf("%q is not a valid subdomain: use up to %d lowercase letters, digits and hyphens", requested, subdomain.MaxLength)
	} else {
		sub = requested
		tun, err = s.ClaimTunnel(sub, tunnelListener, bindAddr, bindPort, clientIP)
	}
	if err != nil {
		log.Printf("Tunnel request from %s rejected: %v", sshConn.RemoteAddr(), err)
		fmt.Fprintf(channel, "\r\n  ERROR: %s\r\n\r\n", err)
		channel.Close()
		return
	}

	tun.SetSSHConn(sshConn)
	defer s.RemoveTunnel(sub)

	log.Printf("New SSH connection from %s, serving subdomain: %s", sshConn.RemoteAddr(), sub)

	url := fmt.Sprintf("https://%s.%s", sub, s.domain)

	// ANSI color codes
	const (
		reset     = "\033[0m"
		gray      = "\033[38;5;245m"
		boldGreen = "\033[1;32m"
		purple    = "\033[38;5;141m"
	)

	urlMessage := "\r\n" +
		gray + "Connected to " + s.domain + "." + reset + "\r\n" +
		boldGreen + "Tunnel is live!" + reset + "\r\n" +
		gray + "Public URL: " + purple + url + reset + "\r\n\r\n"

	fmt.Fprint(channel, urlMessage)

	logger := tunnel.NewRequestLogger(channel, config.LogBufferSize)
	tun.SetLogger(logger)
	defer logger.Close()

	// Accept connections on the tunnel listener
	go func() {
		for {
			tcpConn, err := tunnelListener.Accept()
			if err != nil {
				return
			}
			go s.forwardToSSH(sshConn, tcpConn, tun)
		}
	}()

	// Handle remaining session requests
	go func(reqs <-chan *ssh.Request) {
		for req := range reqs {
			switch req.Type {
			case "signal":
				if req.WantReply {
					req.Reply(true, nil)
				}
				sshConn.Close()
				return
			default:
				if req.WantReply {
					req.Reply(false, nil)
				}
			}
		}
	}(requests)

	// Read from channel to detect disconnect or Ctrl+C
	buf := make([]byte, 1)
	for {
		_, err := channel.Read(buf)
		if err != nil {
			break
		}
		if buf[0] == 0x03 { // Ctrl+C
			sshConn.Close()
			break
		}
	}

	log.Printf("SSH connection closed for subdomain: %s", sub)
}

// awaitCommand waits for the client's exec or shell request and returns the
// command it carried, empty for a plain shell. Requests that arrive first
// (pty-req, env) are answered on the way past.
func awaitCommand(reqs <-chan *ssh.Request) (string, bool) {
	timeout := time.After(5 * time.Second)
	for {
		select {
		case req, ok := <-reqs:
			if !ok {
				return "", false
			}
			switch req.Type {
			case "exec":
				var payload execRequest
				if err := ssh.Unmarshal(req.Payload, &payload); err != nil {
					if req.WantReply {
						req.Reply(false, nil)
					}
					return "", false
				}
				if req.WantReply {
					req.Reply(true, nil)
				}
				return strings.TrimSpace(payload.Command), true
			case "shell":
				if req.WantReply {
					req.Reply(true, nil)
				}
				return "", true
			case "pty-req", "env":
				if req.WantReply {
					req.Reply(true, nil)
				}
			default:
				if req.WantReply {
					req.Reply(false, nil)
				}
			}
		case <-timeout:
			return "", false
		}
	}
}

// sendErrorAndClose sends an error message to the client and closes the connection
// This is used when the connection is rejected after SSH handshake (e.g., IP blocked)

func (s *Server) forwardToSSH(sshConn *ssh.ServerConn, tcpConn net.Conn, tun *tunnel.Tunnel) {
	defer tcpConn.Close()

	var originAddr string
	var originPort uint32
	if tcpAddr, ok := tcpConn.RemoteAddr().(*net.TCPAddr); ok {
		originAddr = tcpAddr.IP.String()
		originPort = uint32(tcpAddr.Port)
	} else {
		originAddr = "0.0.0.0"
		originPort = 0
	}

	channel, reqs, err := sshConn.OpenChannel("forwarded-tcpip", ssh.Marshal(&forwardedTCPPayload{
		Addr:       tun.BindAddr,
		Port:       tun.BindPort,
		OriginAddr: originAddr,
		OriginPort: originPort,
	}))
	if err != nil {
		log.Printf("Failed to open forwarded-tcpip channel: %v", err)
		return
	}
	defer channel.Close()

	go ssh.DiscardRequests(reqs)

	// Copy data bidirectionally. When one direction completes (or errors),
	// close the write side to signal the other goroutine to finish.
	done := make(chan struct{})
	go func() {
		io.Copy(channel, tcpConn)
		// Signal SSH channel we're done sending
		channel.CloseWrite()
	}()
	go func() {
		defer close(done)
		io.Copy(tcpConn, channel)
	}()
	<-done
}

// formatDuration formats a duration as a human-readable string (e.g., "2h", "45m")
