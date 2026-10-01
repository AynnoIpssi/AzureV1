// Vecteurs officiels : NIST (SHA-256), RFC 4231 (HMAC), RFC 7914 section 11
// (PBKDF2-HMAC-SHA256), RFC 8439 (ChaCha20, Poly1305, AEAD).
use azure_stockage::crypto::{aead, chacha20, hmac, poly1305, sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    let clean: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (0..clean.len()).step_by(2).map(|i| u8::from_str_radix(&clean[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn sha256_nist() {
    assert_eq!(hex(&sha256::sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(hex(&sha256::sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(
        hex(&sha256::sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(hex(&sha256::sha256(&vec![b'a'; 1_000_000])), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
}

#[test]
fn hmac_rfc4231() {
    assert_eq!(
        hex(&hmac::hmac_sha256(&[0x0b; 20], b"Hi There")),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
    assert_eq!(
        hex(&hmac::hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
    // Cle plus longue qu'un bloc (cas 6).
    assert_eq!(
        hex(&hmac::hmac_sha256(&[0xaa; 131], b"Test Using Larger Than Block-Size Key - Hash Key First")),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    );
}

#[test]
fn pbkdf2_rfc7914() {
    let mut out = [0u8; 64];
    hmac::pbkdf2_sha256(b"passwd", b"salt", 1, &mut out);
    assert_eq!(
        hex(&out),
        "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"
    );
    hmac::pbkdf2_sha256(b"Password", b"NaCl", 80000, &mut out);
    assert_eq!(
        hex(&out),
        "4ddcd8f60b98be21830cee5ef22701f9641a4418d04c0414aeff08876b34ab56a1d425a1225833549adb841b51c9b3176a272bdebba1d078478f62b397f33c8d"
    );
}

#[test]
fn chacha20_block_rfc8439() {
    let key: [u8; 32] = std::array::from_fn(|i| i as u8);
    let nonce: [u8; 12] = unhex("000000090000004a00000000").try_into().unwrap();
    assert_eq!(
        hex(&chacha20::block(&key, 1, &nonce)),
        "10f1e7e4d13b5915500fdd1fa32071c4c7d1f4c733c068030422aa9ac3d46c4e\
         d2826446079faa0914c2d705d98b02a2b5129cd1de164eb9cbd083e8a2503c4e"
    );
}

#[test]
fn poly1305_rfc8439() {
    let key: [u8; 32] = unhex("85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b").try_into().unwrap();
    assert_eq!(hex(&poly1305::poly1305(&key, b"Cryptographic Forum Research Group")), "a8061dc1305136c6c22b8baf0c0127a9");
}

#[test]
fn aead_rfc8439() {
    let key: [u8; 32] = std::array::from_fn(|i| 0x80 + i as u8);
    let nonce: [u8; 12] = unhex("070000004041424344454647").try_into().unwrap();
    let aad = unhex("50515253c0c1c2c3c4c5c6c7");
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let sealed = aead::encrypt(&key, &nonce, &aad, plaintext);
    assert_eq!(hex(&sealed[..16]), "d31a8d34648e60db7b86afbc53ef7ec2");
    assert_eq!(hex(&sealed[sealed.len() - 16..]), "1ae10b594f09e26a7e902ecbd0600691");
    assert_eq!(aead::decrypt(&key, &nonce, &aad, &sealed).as_deref(), Some(&plaintext[..]));

    let mut altered = sealed.clone();
    altered[3] ^= 1;
    assert_eq!(aead::decrypt(&key, &nonce, &aad, &altered), None, "un octet modifie doit etre refuse");
    assert_eq!(aead::decrypt(&key, &nonce, b"autre contexte", &sealed), None);
}

#[test]
fn seal_uses_a_fresh_nonce_each_time() {
    let key = [7u8; 32];
    let (a, b) = (aead::seal(&key, b"ctx", b"secret").unwrap(), aead::seal(&key, b"ctx", b"secret").unwrap());
    assert_ne!(a, b);
    assert_eq!(aead::open(&key, b"ctx", &a).unwrap(), b"secret");
    assert!(aead::open(&[8u8; 32], b"ctx", &a).is_err());
}
