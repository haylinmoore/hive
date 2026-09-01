package tunnel

import (
	"context"
	"net"
	"net/http"
	"sync"
	"time"
)

// SSHCloser is an interface for closing SSH connections
type SSHCloser interface {
	Close() error
}

// Tunnel represents an active SSH tunnel
type Tunnel struct {
	Subdomain string
	Listener  net.Listener
	CreatedAt time.Time
	BindAddr  string
	BindPort  uint32
	ClientIP  string // SSH client IP that created this tunnel
	mu        sync.Mutex
	sshConn   SSHCloser       // Reference to SSH connection for forced closure
	transport *http.Transport // Reusable HTTP transport for proxying
	logger    *RequestLogger  // Async request logger for SSH terminal output
}

// New creates a new tunnel with the given parameters
func New(subdomain string, listener net.Listener, bindAddr string, bindPort uint32, clientIP string) *Tunnel {
	listenerAddr := listener.Addr().String()
	return &Tunnel{
		Subdomain: subdomain,
		Listener:  listener,
		CreatedAt: time.Now(),
		BindAddr:  bindAddr,
		BindPort:  bindPort,
		ClientIP:  clientIP,
		transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				return net.DialTimeout("tcp", listenerAddr, 10*time.Second)
			},
			MaxIdleConns:    10,
			IdleConnTimeout: 90 * time.Second,
		},
	}
}

// Touch updates the last active timestamp

// IsExpired returns true if the tunnel has been inactive for too long or exceeded max lifetime

// TimeRemaining returns the time remaining before the tunnel expires (either by inactivity or max lifetime)

// SetSSHConn sets the SSH connection reference for forced closure
func (t *Tunnel) SetSSHConn(conn SSHCloser) {
	t.mu.Lock()
	t.sshConn = conn
	t.mu.Unlock()
}

// CloseSSH closes the SSH connection associated with this tunnel
func (t *Tunnel) CloseSSH() {
	t.mu.Lock()
	conn := t.sshConn
	t.sshConn = nil // Prevent redundant close calls
	t.mu.Unlock()

	if conn != nil {
		conn.Close()
	}
}

// SetLogger sets the request logger for SSH terminal output
func (t *Tunnel) SetLogger(l *RequestLogger) {
	t.mu.Lock()
	t.logger = l
	t.mu.Unlock()
}

// Logger returns the request logger, or nil if none is set
func (t *Tunnel) Logger() *RequestLogger {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.logger
}

// Transport returns the reusable HTTP transport for this tunnel
func (t *Tunnel) Transport() *http.Transport {
	return t.transport
}

// Close closes the tunnel's listener and cleans up the transport and logger
func (t *Tunnel) Close() {
	t.Listener.Close()
	if t.transport != nil {
		t.transport.CloseIdleConnections()
	}
	t.mu.Lock()
	l := t.logger
	t.logger = nil
	t.mu.Unlock()
	if l != nil {
		l.Close()
	}
}
