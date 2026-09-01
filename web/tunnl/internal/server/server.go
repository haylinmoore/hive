package server

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/pem"
	"fmt"
	"log"
	"net"
	"os"
	"sync"

	"github.com/mikesmitty/edkey"
	"golang.org/x/crypto/ssh"

	"tunnl.gg/internal/subdomain"
	"tunnl.gg/internal/tunnel"
)

// Server manages SSH tunnels and HTTP proxying
type Server struct {
	tunnels   map[string]*tunnel.Tunnel
	mu        sync.RWMutex
	sshConfig *ssh.ServerConfig
	domain    string

	// Stats
	totalConnections uint64
	totalRequests    uint64
}

// New creates a new server instance
func New(hostKeyPath string, domain string, authorizedKeysPath string) (*Server, error) {
	s := &Server{
		tunnels: make(map[string]*tunnel.Tunnel),
		domain:  domain,
	}

	keys, err := loadAuthorizedKeys(authorizedKeysPath)
	if err != nil {
		return nil, err
	}
	log.Printf("Loaded %d authorized key(s) from %s", len(keys), authorizedKeysPath)

	s.sshConfig = &ssh.ServerConfig{
		PublicKeyCallback: func(conn ssh.ConnMetadata, key ssh.PublicKey) (*ssh.Permissions, error) {
			comment, ok := keys.lookup(key)
			if !ok {
				log.Printf("Rejected %s from %s: key not authorized", ssh.FingerprintSHA256(key), conn.RemoteAddr())
				return nil, fmt.Errorf("key not authorized")
			}
			return &ssh.Permissions{Extensions: map[string]string{"key-comment": comment}}, nil
		},
	}

	hostKey, err := loadOrGenerateHostKey(hostKeyPath)
	if err != nil {
		return nil, fmt.Errorf("failed to load host key: %w", err)
	}
	s.sshConfig.AddHostKey(hostKey)

	return s, nil
}

// Domain returns the configured domain
func (s *Server) Domain() string {
	return s.domain
}

// SSHConfig returns the SSH server configuration
func (s *Server) SSHConfig() *ssh.ServerConfig {
	return s.sshConfig
}

func loadOrGenerateHostKey(path string) (ssh.Signer, error) {
	if _, err := os.Stat(path); os.IsNotExist(err) {
		log.Printf("Generating new host key at %s", path)

		_, priv, err := ed25519.GenerateKey(rand.Reader)
		if err != nil {
			return nil, err
		}

		pemBlock := &pem.Block{
			Type:  "OPENSSH PRIVATE KEY",
			Bytes: edkey.MarshalED25519PrivateKey(priv),
		}

		if err := os.WriteFile(path, pem.EncodeToMemory(pemBlock), 0600); err != nil {
			return nil, err
		}
	}

	keyBytes, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	return ssh.ParsePrivateKey(keyBytes)
}

// GenerateUniqueSubdomain generates a subdomain that doesn't collide with existing ones

// CheckAndReserveConnection checks if a new connection from the given IP is allowed
// and atomically reserves a slot if allowed. Returns true if reservation was made.
// Caller MUST call DecrementIPConnection when done if this returns nil.

// BlockIP blocks an IP address

// DecrementIPConnection decrements the connection count for an IP

// RegisterTunnel registers a new tunnel

// ClaimTunnel registers a tunnel under sub, failing if sub is already taken.
func (s *Server) ClaimTunnel(sub string, listener net.Listener, bindAddr string, bindPort uint32, clientIP string) (*tunnel.Tunnel, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	if _, exists := s.tunnels[sub]; exists {
		return nil, fmt.Errorf("subdomain %q is already in use", sub)
	}

	t := tunnel.New(sub, listener, bindAddr, bindPort, clientIP)
	s.tunnels[sub] = t
	return t, nil
}

// ClaimGenerated claims a tunnel under a randomly generated subdomain.
func (s *Server) ClaimGenerated(listener net.Listener, bindAddr string, bindPort uint32, clientIP string) (string, *tunnel.Tunnel, error) {
	const maxAttempts = 10
	for i := 0; i < maxAttempts; i++ {
		sub, err := subdomain.Generate()
		if err != nil {
			return "", nil, err
		}
		if t, err := s.ClaimTunnel(sub, listener, bindAddr, bindPort, clientIP); err == nil {
			return sub, t, nil
		}
	}
	return "", nil, fmt.Errorf("failed to find a free subdomain after %d attempts", maxAttempts)
}

// RemoveTunnel removes and closes a tunnel
func (s *Server) RemoveTunnel(sub string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if t, ok := s.tunnels[sub]; ok {
		t.Close()
		delete(s.tunnels, sub)
	}
}

// GetTunnel retrieves a tunnel by subdomain
func (s *Server) GetTunnel(sub string) *tunnel.Tunnel {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.tunnels[sub]
}

// RegisterSSHConn registers an SSH connection for an IP (for forced closure on block)

// UnregisterSSHConn removes an SSH connection from tracking

// CloseAllForIP closes all SSH connections for a specific IP
// Closing SSH connections triggers cleanup which removes tunnels via defers
// Returns the number of connections closed

// Stop gracefully stops the server's background goroutines
