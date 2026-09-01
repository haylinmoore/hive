package tunnel

import (
	"bytes"
	"errors"
	"net"
	"sync"
	"testing"
)

func newTestTunnel(t *testing.T) *Tunnel {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create test listener: %v", err)
	}
	t.Cleanup(func() { ln.Close() })
	return New("test-sub-00000000", ln, "127.0.0.1", 8080, "127.0.0.1")
}

func TestTransport(t *testing.T) {
	tun := newTestTunnel(t)
	tr := tun.Transport()
	if tr == nil {
		t.Error("Transport() returned nil")
	}
}

type mockSSHConn struct {
	mu     sync.Mutex
	closed bool
}

func (m *mockSSHConn) Close() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.closed {
		return errors.New("already closed")
	}
	m.closed = true
	return nil
}

func (m *mockSSHConn) isClosed() bool {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.closed
}

func TestSetSSHConn(t *testing.T) {
	tun := newTestTunnel(t)
	mock := &mockSSHConn{}
	tun.SetSSHConn(mock)

	tun.mu.Lock()
	got := tun.sshConn
	tun.mu.Unlock()

	if got != mock {
		t.Error("SetSSHConn() did not set sshConn")
	}
}

func TestCloseSSH(t *testing.T) {
	tun := newTestTunnel(t)
	mock := &mockSSHConn{}
	tun.SetSSHConn(mock)

	tun.CloseSSH()

	if !mock.isClosed() {
		t.Error("CloseSSH() did not close the SSH connection")
	}

	// sshConn should be nil after close (prevents double-close)
	tun.mu.Lock()
	got := tun.sshConn
	tun.mu.Unlock()
	if got != nil {
		t.Error("CloseSSH() should nil out sshConn")
	}
}

func TestCloseSSH_Nil(t *testing.T) {
	tun := newTestTunnel(t)
	// Should not panic when no SSH connection is set
	tun.CloseSSH()
}

func TestSetLogger(t *testing.T) {
	tun := newTestTunnel(t)
	var buf bytes.Buffer
	logger := NewRequestLogger(&buf, 16)
	defer logger.Close()

	tun.SetLogger(logger)

	got := tun.Logger()
	if got != logger {
		t.Error("SetLogger()/Logger() round-trip failed")
	}
}

func TestLogger_NilByDefault(t *testing.T) {
	tun := newTestTunnel(t)
	if tun.Logger() != nil {
		t.Error("Logger() should be nil by default")
	}
}

func TestClose_ClosesLogger(t *testing.T) {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create listener: %v", err)
	}
	tun := New("test-sub-00000000", ln, "127.0.0.1", 8080, "127.0.0.1")

	var buf bytes.Buffer
	logger := NewRequestLogger(&buf, 16)
	tun.SetLogger(logger)

	tun.Close()

	// After Close, logger should be nil
	if tun.Logger() != nil {
		t.Error("Close() should nil out logger")
	}
}

func TestClose(t *testing.T) {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to create listener: %v", err)
	}
	tun := New("test-sub-00000000", ln, "127.0.0.1", 8080, "127.0.0.1")
	tun.Close()

	// Listener should be closed — Accept should fail
	_, err = ln.Accept()
	if err == nil {
		t.Error("Close() should close the listener")
	}
}
