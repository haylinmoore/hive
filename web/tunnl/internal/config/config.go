package config

import (
	"fmt"
	"time"
)

const (
	DefaultDomain = "tunnl.gg"

	// SSH handshake timeout
	SSHHandshakeTimeout = 30 * time.Second

	// HTTP server timeouts
	HTTPReadHeaderTimeout  = 5 * time.Second
	HTTPReadTimeout        = 10 * time.Second
	HTTPWriteTimeout       = 10 * time.Second
	HTTPIdleTimeout        = 30 * time.Second
	HTTPSReadHeaderTimeout = 5 * time.Second
	HTTPSReadTimeout       = 30 * time.Second
	HTTPSWriteTimeout      = 30 * time.Second
	HTTPSIdleTimeout       = 120 * time.Second
	StatsReadHeaderTimeout = 2 * time.Second
	StatsReadTimeout       = 5 * time.Second
	StatsWriteTimeout      = 5 * time.Second
	ShutdownTimeout        = 10 * time.Second

	// WebSocket limits
	WebSocketIdleTimeout = 2 * time.Hour

	// Request logging
	LogBufferSize = 128 // buffered channel size for SSH terminal request logs
)

// Config holds runtime configuration loaded from environment
type Config struct {
	SSHAddr            string
	HTTPAddr           string
	HTTPSAddr          string
	StatsAddr          string
	HostKeyPath        string
	AuthorizedKeysPath string
	TLSCert            string
	TLSKey             string
	Domain             string
}

// Default returns configuration with default values
func Default() *Config {
	return &Config{
		SSHAddr:            ":22",
		HTTPAddr:           ":80",
		HTTPSAddr:          ":443",
		StatsAddr:          "127.0.0.1:9090",
		HostKeyPath:        "host_key",
		AuthorizedKeysPath: "authorized_keys",
		TLSCert:            fmt.Sprintf("/etc/letsencrypt/live/%s/fullchain.pem", DefaultDomain),
		TLSKey:             fmt.Sprintf("/etc/letsencrypt/live/%s/privkey.pem", DefaultDomain),
		Domain:             DefaultDomain,
	}
}
