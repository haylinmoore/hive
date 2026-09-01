package server

import (
	"net"
	"testing"
	"time"

	"golang.org/x/crypto/ssh"
)

func testListener(t *testing.T) net.Listener {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}
	t.Cleanup(func() { l.Close() })
	return l
}

func TestClaimTunnel(t *testing.T) {
	s := newTestServer(t)

	if _, err := s.ClaimTunnel("myapp", testListener(t), "", 80, "127.0.0.1"); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if s.GetTunnel("myapp") == nil {
		t.Fatal("claimed tunnel is not reachable")
	}

	if _, err := s.ClaimTunnel("myapp", testListener(t), "", 80, "127.0.0.1"); err == nil {
		t.Error("expected the second claim on myapp to fail")
	}

	s.RemoveTunnel("myapp")
	if _, err := s.ClaimTunnel("myapp", testListener(t), "", 80, "127.0.0.1"); err != nil {
		t.Errorf("expected myapp to be claimable after release: %v", err)
	}
}

func TestClaimGenerated(t *testing.T) {
	s := newTestServer(t)

	sub, tun, err := s.ClaimGenerated(testListener(t), "", 80, "127.0.0.1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if tun == nil {
		t.Fatal("expected a tunnel")
	}
	if s.GetTunnel(sub) != tun {
		t.Errorf("generated subdomain %q does not resolve to the claimed tunnel", sub)
	}
}

func TestAwaitCommand(t *testing.T) {
	t.Run("exec carries the subdomain", func(t *testing.T) {
		reqs := make(chan *ssh.Request, 2)
		reqs <- &ssh.Request{Type: "pty-req"}
		reqs <- &ssh.Request{Type: "exec", Payload: ssh.Marshal(&execRequest{Command: " myapp "})}

		got, ok := awaitCommand(reqs)
		if !ok {
			t.Fatal("expected awaitCommand to succeed")
		}
		if got != "myapp" {
			t.Errorf("got %q, want myapp", got)
		}
	})

	t.Run("shell means no preference", func(t *testing.T) {
		reqs := make(chan *ssh.Request, 1)
		reqs <- &ssh.Request{Type: "shell"}

		got, ok := awaitCommand(reqs)
		if !ok {
			t.Fatal("expected awaitCommand to succeed")
		}
		if got != "" {
			t.Errorf("got %q, want an empty command", got)
		}
	})

	t.Run("closed channel", func(t *testing.T) {
		reqs := make(chan *ssh.Request)
		close(reqs)

		if _, ok := awaitCommand(reqs); ok {
			t.Error("expected awaitCommand to fail on a closed channel")
		}
	})
}

func TestAwaitCommandTimeout(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping timeout test in short mode")
	}
	start := time.Now()
	if _, ok := awaitCommand(make(chan *ssh.Request)); ok {
		t.Error("expected awaitCommand to time out")
	}
	if elapsed := time.Since(start); elapsed < 4*time.Second {
		t.Errorf("returned after %v, expected to wait about 5s", elapsed)
	}
}
