// Package tlscert serves a certificate that can be replaced on disk while the
// server is running, so a renewal does not need a restart and does not drop
// live tunnels.
package tlscert

import (
	"crypto/tls"
	"fmt"
	"log"
	"os"
	"sync"
	"time"
)

// Reloader holds the parsed certificate and reloads it when the file on disk
// changes. ACME clients replace these files in place, and a renewal that
// required a restart would take every tunnel down with it.
type Reloader struct {
	certPath string
	keyPath  string

	mu      sync.RWMutex
	cert    *tls.Certificate
	modTime time.Time
}

// New returns a Reloader for the given pair. A certificate that cannot be read
// yet is not fatal: issuance may still be in flight, and the next handshake
// tries again.
func New(certPath, keyPath string) *Reloader {
	r := &Reloader{certPath: certPath, keyPath: keyPath}
	if err := r.reload(); err != nil {
		log.Printf("Certificate not usable yet, will retry on each handshake: %v", err)
	}
	return r
}

// GetCertificate satisfies tls.Config.GetCertificate.
func (r *Reloader) GetCertificate(*tls.ClientHelloInfo) (*tls.Certificate, error) {
	if r.changed() {
		if err := r.reload(); err != nil {
			log.Printf("Failed to reload certificate, serving the previous one: %v", err)
		}
	}

	r.mu.RLock()
	defer r.mu.RUnlock()
	if r.cert == nil {
		return nil, fmt.Errorf("no certificate available")
	}
	return r.cert, nil
}

// changed reports whether the certificate file has been written since the copy
// in memory was parsed.
func (r *Reloader) changed() bool {
	info, err := os.Stat(r.certPath)
	if err != nil {
		return false
	}

	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.cert == nil || !info.ModTime().Equal(r.modTime)
}

func (r *Reloader) reload() error {
	info, err := os.Stat(r.certPath)
	if err != nil {
		return err
	}

	cert, err := tls.LoadX509KeyPair(r.certPath, r.keyPath)
	if err != nil {
		return err
	}

	r.mu.Lock()
	defer r.mu.Unlock()
	// Another handshake may have won the race; the newest mtime wins.
	if r.cert != nil && info.ModTime().Before(r.modTime) {
		return nil
	}
	r.cert = &cert
	r.modTime = info.ModTime()
	log.Printf("Loaded certificate from %s", r.certPath)
	return nil
}
