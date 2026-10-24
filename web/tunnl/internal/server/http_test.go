package server

import (
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"
	"time"

	"tunnl.gg/internal/config"
)

func TestStripPort(t *testing.T) {
	tests := []struct {
		name  string
		input string
		want  string
	}{
		{"with port", "example.com:443", "example.com"},
		{"without port", "example.com", "example.com"},
		{"ipv4 with port", "127.0.0.1:8080", "127.0.0.1"},
		{"ipv6 with port", "[::1]:8080", "::1"},
		{"bracketed ipv6 without port", "[::1]", "::1"},
		{"ipv6 without port", "::1", "::1"},
		{"empty string", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := stripPort(tt.input); got != tt.want {
				t.Errorf("stripPort(%q) = %q, want %q", tt.input, got, tt.want)
			}
		})
	}
}

func TestIsWebSocketRequest(t *testing.T) {
	tests := []struct {
		name       string
		upgrade    string
		connection string
		want       bool
	}{
		{"valid websocket", "websocket", "Upgrade", true},
		{"case insensitive", "WebSocket", "upgrade", true},
		{"missing upgrade header", "", "Upgrade", false},
		{"missing connection header", "websocket", "", false},
		{"wrong upgrade value", "http/2", "Upgrade", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := &http.Request{Header: http.Header{}}
			if tt.upgrade != "" {
				r.Header.Set("Upgrade", tt.upgrade)
			}
			if tt.connection != "" {
				r.Header.Set("Connection", tt.connection)
			}
			if got := isWebSocketRequest(r); got != tt.want {
				t.Errorf("isWebSocketRequest() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestSetSecurityHeaders(t *testing.T) {
	w := httptest.NewRecorder()
	setSecurityHeaders(w)

	expected := map[string]string{
		"X-Content-Type-Options": "nosniff",
		"X-Frame-Options":        "DENY",
		"X-Xss-Protection":       "1; mode=block",
		"Referrer-Policy":        "strict-origin-when-cross-origin",
	}

	for header, want := range expected {
		if got := w.Header().Get(header); got != want {
			t.Errorf("header %q = %q, want %q", header, got, want)
		}
	}
}

func newTestServer(t *testing.T) *Server {
	t.Helper()
	dir := t.TempDir()
	if err := os.WriteFile(dir+"/authorized_keys", []byte(testAuthorizedKey), 0600); err != nil {
		t.Fatalf("failed to write authorized keys: %v", err)
	}
	s, err := New(dir+"/host_key", config.DefaultDomain, dir+"/authorized_keys")
	if err != nil {
		t.Fatalf("failed to create test server: %v", err)
	}
	return s
}

func TestHTTPRedirectHandler(t *testing.T) {
	s := newTestServer(t)
	handler := s.HTTPRedirectHandler()

	tests := []struct {
		name       string
		host       string
		path       string
		wantCode   int
		wantTarget string
	}{
		{
			"subdomain redirect",
			"test-sub-12345678.tunnl.gg",
			"/foo",
			http.StatusMovedPermanently,
			"https://test-sub-12345678.tunnl.gg/foo",
		},
		{
			"bare domain redirect",
			"tunnl.gg",
			"/",
			http.StatusMovedPermanently,
			"https://tunnl.gg/",
		},
		{
			"bad domain rejected",
			"evil.com",
			"/",
			http.StatusBadRequest,
			"",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := httptest.NewRequest("GET", "http://"+tt.host+tt.path, nil)
			r.Host = tt.host
			w := httptest.NewRecorder()
			handler.ServeHTTP(w, r)

			if w.Code != tt.wantCode {
				t.Errorf("status = %d, want %d", w.Code, tt.wantCode)
			}
			if tt.wantTarget != "" {
				loc := w.Header().Get("Location")
				if loc != tt.wantTarget {
					t.Errorf("Location = %q, want %q", loc, tt.wantTarget)
				}
			}
		})
	}
}

func TestStatusCaptureWriter(t *testing.T) {
	t.Run("captures explicit status", func(t *testing.T) {
		rec := httptest.NewRecorder()
		sw := &statusCaptureWriter{ResponseWriter: rec}
		sw.WriteHeader(http.StatusNotFound)

		if sw.status != http.StatusNotFound {
			t.Errorf("status = %d, want %d", sw.status, http.StatusNotFound)
		}
	})

	t.Run("defaults to 200 on Write", func(t *testing.T) {
		rec := httptest.NewRecorder()
		sw := &statusCaptureWriter{ResponseWriter: rec}
		sw.Write([]byte("hello"))

		if sw.status != http.StatusOK {
			t.Errorf("status = %d, want %d", sw.status, http.StatusOK)
		}
	})

	t.Run("first WriteHeader wins", func(t *testing.T) {
		rec := httptest.NewRecorder()
		sw := &statusCaptureWriter{ResponseWriter: rec}
		sw.WriteHeader(http.StatusCreated)
		sw.WriteHeader(http.StatusNotFound)

		if sw.status != http.StatusCreated {
			t.Errorf("status = %d, want %d (first call should win)", sw.status, http.StatusCreated)
		}
	})

	t.Run("Unwrap returns inner writer", func(t *testing.T) {
		rec := httptest.NewRecorder()
		sw := &statusCaptureWriter{ResponseWriter: rec}

		if sw.Unwrap() != rec {
			t.Error("Unwrap() should return the underlying ResponseWriter")
		}
	})
}

func TestCopyIdleTimeout(t *testing.T) {
	t.Run("copies until EOF", func(t *testing.T) {
		src, srcWriter := net.Pipe()
		dst, dstReader := net.Pipe()
		defer src.Close()
		defer dst.Close()

		payload := strings.Repeat("x", 128*1024)
		go func() {
			io.WriteString(srcWriter, payload)
			srcWriter.Close()
		}()

		received := make(chan int64, 1)
		go func() {
			n, _ := io.Copy(io.Discard, dstReader)
			received <- n
		}()

		written, err := copyIdleTimeout(dst, src, time.Second)
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if written != int64(len(payload)) {
			t.Errorf("copied %d bytes, want %d", written, len(payload))
		}
		dst.Close()
		if got := <-received; got != int64(len(payload)) {
			t.Errorf("destination saw %d bytes, want %d", got, len(payload))
		}
	})

	t.Run("gives up when idle", func(t *testing.T) {
		src, srcWriter := net.Pipe()
		dst, dstReader := net.Pipe()
		defer src.Close()
		defer dst.Close()
		defer srcWriter.Close()

		go io.Copy(io.Discard, dstReader)

		if _, err := copyIdleTimeout(dst, src, 50*time.Millisecond); err == nil {
			t.Error("expected a timeout error, got nil")
		}
	})
}

func TestProxyTellsTheAppItIsBehindHTTPS(t *testing.T) {
	var got *http.Request
	var body string
	backend := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		got = r
		b, _ := io.ReadAll(r.Body)
		body = string(b)
		http.SetCookie(w, &http.Cookie{Name: "session", Value: "new", Secure: true})
	}))
	defer backend.Close()

	target := strings.TrimPrefix(backend.URL, "http://")
	req := httptest.NewRequest("POST", "https://crucible.tunnl.hayl.in/login", strings.NewReader("user=haylin"))
	req.RemoteAddr = "203.0.113.7:51000"
	req.Header.Set("Cookie", "session=old")
	req.Header.Set("Origin", "https://crucible.tunnl.hayl.in")
	req.Header.Set("X-Forwarded-For", "10.9.9.9")
	req.Header.Set("X-Forwarded-Proto", "http")
	rec := httptest.NewRecorder()

	newProxy(target, http.DefaultTransport, "crucible").ServeHTTP(rec, req)

	if got == nil {
		t.Fatalf("backend never saw the request, status %d", rec.Code)
	}
	checks := map[string]string{
		"Host":              got.Host,
		"Cookie":            got.Header.Get("Cookie"),
		"Origin":            got.Header.Get("Origin"),
		"X-Forwarded-Proto": got.Header.Get("X-Forwarded-Proto"),
		"X-Forwarded-Host":  got.Header.Get("X-Forwarded-Host"),
		"X-Forwarded-For":   got.Header.Get("X-Forwarded-For"),
		"body":              body,
		"method":            got.Method,
		"path":              got.URL.Path,
	}
	want := map[string]string{
		"Host":              "crucible.tunnl.hayl.in",
		"Cookie":            "session=old",
		"Origin":            "https://crucible.tunnl.hayl.in",
		"X-Forwarded-Proto": "https",
		"X-Forwarded-Host":  "crucible.tunnl.hayl.in",
		"X-Forwarded-For":   "203.0.113.7",
		"body":              "user=haylin",
		"method":            "POST",
		"path":              "/login",
	}
	for k, w := range want {
		if checks[k] != w {
			t.Errorf("%s = %q, want %q", k, checks[k], w)
		}
	}
	if sc := rec.Header().Get("Set-Cookie"); !strings.Contains(sc, "session=new") {
		t.Errorf("Set-Cookie did not come back to the client: %q", sc)
	}
}

func TestSetForwardedReplacesWhatTheClientClaimed(t *testing.T) {
	r := httptest.NewRequest("GET", "https://crucible.tunnl.hayl.in/socket", nil)
	r.RemoteAddr = "[2001:db8::1]:4000"
	r.Header.Set("X-Forwarded-For", "10.9.9.9")
	r.Header.Set("X-Forwarded-Proto", "http")
	setForwarded(r)

	if v := r.Header.Get("X-Forwarded-For"); v != "2001:db8::1" {
		t.Errorf("X-Forwarded-For = %q", v)
	}
	if v := r.Header.Get("X-Forwarded-Proto"); v != "https" {
		t.Errorf("X-Forwarded-Proto = %q", v)
	}
	if v := r.Header.Get("X-Forwarded-Host"); v != "crucible.tunnl.hayl.in" {
		t.Errorf("X-Forwarded-Host = %q", v)
	}
}
