use curve25519_dalek::{ristretto::RistrettoPoint, scalar::Scalar};
use merlin::Transcript;

pub trait TranscriptExt {
    fn append_scalar(&mut self, label: &'static [u8], s: &Scalar);
    fn append_point(&mut self, label: &'static [u8], p: &RistrettoPoint);
    fn challenge_scalar(&mut self, label: &'static [u8]) -> Scalar;
    fn challenge_point(&mut self, label: &'static [u8]) -> RistrettoPoint;
}

impl TranscriptExt for Transcript {
    fn append_scalar(&mut self, label: &'static [u8], s: &Scalar) {
        self.append_message(label, s.as_bytes());
    }

    fn append_point(&mut self, label: &'static [u8], p: &RistrettoPoint) {
        self.append_message(label, p.compress().as_bytes());
    }

    fn challenge_scalar(&mut self, label: &'static [u8]) -> Scalar {
        let mut buf = [0u8; 64];
        self.challenge_bytes(label, &mut buf);
        Scalar::from_bytes_mod_order_wide(&buf)
    }
    fn challenge_point(&mut self, label: &'static [u8]) -> RistrettoPoint {
        let mut buf = [0u8; 64];
        self.challenge_bytes(label, &mut buf);
        RistrettoPoint::from_uniform_bytes(&buf)
    }
}
