package tlscert

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/pem"
	"math/big"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// writePair writes a self-signed certificate for cn and returns the two paths.
func writePair(t *testing.T, dir, cn string) (string, string) {
	t.Helper()

	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatalf("failed to generate key: %v", err)
	}
	tmpl := x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject:      pkix.Name{CommonName: cn},
		NotBefore:    time.Now().Add(-time.Hour),
		NotAfter:     time.Now().Add(time.Hour),
		DNSNames:     []string{cn},
	}
	der, err := x509.CreateCertificate(rand.Reader, &tmpl, &tmpl, &key.PublicKey, key)
	if err != nil {
		t.Fatalf("failed to create certificate: %v", err)
	}

	certPath := filepath.Join(dir, "cert.pem")
	keyPath := filepath.Join(dir, "key.pem")

	certPEM := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: der})
	if err := os.WriteFile(certPath, certPEM, 0644); err != nil {
		t.Fatalf("failed to write certificate: %v", err)
	}
	keyDER, err := x509.MarshalECPrivateKey(key)
	if err != nil {
		t.Fatalf("failed to marshal key: %v", err)
	}
	keyPEM := pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: keyDER})
	if err := os.WriteFile(keyPath, keyPEM, 0600); err != nil {
		t.Fatalf("failed to write key: %v", err)
	}
	return certPath, keyPath
}

func commonName(t *testing.T, r *Reloader) string {
	t.Helper()
	cert, err := r.GetCertificate(nil)
	if err != nil {
		t.Fatalf("GetCertificate: %v", err)
	}
	parsed, err := x509.ParseCertificate(cert.Certificate[0])
	if err != nil {
		t.Fatalf("failed to parse served certificate: %v", err)
	}
	return parsed.Subject.CommonName
}

func TestReloadsWhenReplaced(t *testing.T) {
	dir := t.TempDir()
	certPath, keyPath := writePair(t, dir, "placeholder.example")

	r := New(certPath, keyPath)
	if got := commonName(t, r); got != "placeholder.example" {
		t.Fatalf("served %q, want placeholder.example", got)
	}

	// An ACME renewal replaces the files in place. mtime has one-second
	// granularity on some filesystems, so make the change unambiguous.
	time.Sleep(1100 * time.Millisecond)
	writePair(t, dir, "renewed.example")

	if got := commonName(t, r); got != "renewed.example" {
		t.Errorf("served %q after replacement, want renewed.example", got)
	}
}

func TestMissingCertificateIsNotFatal(t *testing.T) {
	dir := t.TempDir()
	r := New(filepath.Join(dir, "cert.pem"), filepath.Join(dir, "key.pem"))

	if _, err := r.GetCertificate(nil); err == nil {
		t.Error("expected an error while no certificate exists")
	}

	// Issuance completes after startup; the next handshake should pick it up.
	certPath, keyPath := writePair(t, dir, "issued.example")
	r.certPath, r.keyPath = certPath, keyPath
	if got := commonName(t, r); got != "issued.example" {
		t.Errorf("served %q, want issued.example", got)
	}
}
