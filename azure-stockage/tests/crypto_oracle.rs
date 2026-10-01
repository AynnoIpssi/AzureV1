// Vecteurs aux limites, calcules par une implementation independante
// (python `cryptography` / hashlib, OpenSSL) : bords de remplissage de
// SHA-256, reductions modulaires de Poly1305 (entrees de RFC 8439 A.3),
// toutes les longueurs utiles de ChaCha20-Poly1305. Calcules le 2026-09-27
// avec python `cryptography` 49 et hashlib ; les entrees sont derivees de
// `det(longueur, graine)` (SHA-256 en chaine), identique des deux cotes.
use azure_stockage::crypto::{aead, chacha20, hmac, poly1305, sha256};

fn unhex(t: &str) -> Vec<u8> {
    (0..t.len()).step_by(2).map(|i| u8::from_str_radix(&t[i..i + 2], 16).unwrap()).collect()
}

fn det(n: usize, seed: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0u32;
    while out.len() < n {
        let mut block = seed.to_vec();
        block.extend_from_slice(&i.to_le_bytes());
        out.extend_from_slice(&sha256::sha256(&block));
        i += 1;
    }
    out.truncate(n);
    out
}

#[test]
fn sha256_padding_boundaries() {
    assert_eq!(sha256::sha256(&det(1, b"sha")).to_vec(), unhex("c4694f2e93d5c4e7d51f9c5deb75e6cc8be5e1114178c6a45b6fc2c566a0aa8c"), "longueur 1");
    assert_eq!(sha256::sha256(&det(55, b"sha")).to_vec(), unhex("6114a53d32e9a26edaefcdf283344d5f3ad34754e49583985bb54097651aab1f"), "longueur 55");
    assert_eq!(sha256::sha256(&det(56, b"sha")).to_vec(), unhex("fe2655e1cd96d59040250c201db28ffb900d0376176d00b9124d839d481e331a"), "longueur 56");
    assert_eq!(sha256::sha256(&det(57, b"sha")).to_vec(), unhex("178c984a748842eb2e0f09d9c904cae0ba9998a1cfe276ed76832176f998d6bd"), "longueur 57");
    assert_eq!(sha256::sha256(&det(63, b"sha")).to_vec(), unhex("40e6494cbf087b03e3af078214ad787ff87c4ede8b957b8dc5bf10d0c9a82ed9"), "longueur 63");
    assert_eq!(sha256::sha256(&det(64, b"sha")).to_vec(), unhex("00024362f076bb006d7507cd50aa2cd8f5f095cea0c4ebd0040931a1306673d7"), "longueur 64");
    assert_eq!(sha256::sha256(&det(65, b"sha")).to_vec(), unhex("112a1a076b1d7a29334f2b7d25d676b0d806fe3a8ce748111b16c6920c97cab1"), "longueur 65");
    assert_eq!(sha256::sha256(&det(119, b"sha")).to_vec(), unhex("3db1c8321195b1516b67eceefd20fe3eff27b3a9ab5a2c20a386a910ec3cfc7e"), "longueur 119");
    assert_eq!(sha256::sha256(&det(120, b"sha")).to_vec(), unhex("d6c94dc10717f75625d5ccd0a8ff308f978602a3bcb32b10642a1e50f52b9280"), "longueur 120");
    assert_eq!(sha256::sha256(&det(127, b"sha")).to_vec(), unhex("401aadc93106829e05302d5ea918a68e8e63039454dcf1327b0bca75f6fca75d"), "longueur 127");
    assert_eq!(sha256::sha256(&det(128, b"sha")).to_vec(), unhex("7c495880b25d0c4c276a6aff593aa3110bdaaf60d1eebf84654a709be657521f"), "longueur 128");
    assert_eq!(sha256::sha256(&det(129, b"sha")).to_vec(), unhex("3b7faf93d8274483ffdcd153dadfe571153b539700a2bdb72edd591695d93a23"), "longueur 129");
    assert_eq!(sha256::sha256(&det(1000, b"sha")).to_vec(), unhex("4dce7bbfb6ec3a19622c014e67310e9e21fc03638537268bcffe60a10833f108"), "longueur 1000");
    // Par morceaux : meme resultat.
    let data = det(1000, b"sha");
    let mut s = sha256::Sha256::new();
    for chunk in data.chunks(7) {
        s.update(chunk);
    }
    assert_eq!(s.finish(), sha256::sha256(&data));
}

#[test]
fn hmac_key_and_message_lengths() {
    assert_eq!(hmac::hmac_sha256(&det(0, b"hk"), &det(0, b"hm")).to_vec(), unhex("b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad"));
    assert_eq!(hmac::hmac_sha256(&det(1, b"hk"), &det(1, b"hm")).to_vec(), unhex("fb0bcbf39567b500fd8d486b7aedb6cba87a978ce55724cfb41d4eaaed489311"));
    assert_eq!(hmac::hmac_sha256(&det(32, b"hk"), &det(100, b"hm")).to_vec(), unhex("924aa3d0e7f17664c9b0d1a77ca1589add92d08d83ab11e9045c120cdb37d09c"));
    assert_eq!(hmac::hmac_sha256(&det(63, b"hk"), &det(64, b"hm")).to_vec(), unhex("d5faab97ca76ab2012400e0cfccd6f6953d7fe489a2736a669c72d049cb50751"));
    assert_eq!(hmac::hmac_sha256(&det(64, b"hk"), &det(64, b"hm")).to_vec(), unhex("41d3045e8eac876bf000e0686de3ea61097f6a3c7de9f5824464c081a85efe2d"));
    assert_eq!(hmac::hmac_sha256(&det(65, b"hk"), &det(65, b"hm")).to_vec(), unhex("dc14d9aa9cb6a16967a1849d6cd61f6750d4c933866332d6844e029218267f44"));
    assert_eq!(hmac::hmac_sha256(&det(200, b"hk"), &det(1000, b"hm")).to_vec(), unhex("1bb73579f15b44cafa49ee3829515ead7a407f62795d944527623528d0d96bf7"));
}

#[test]
fn pbkdf2_lengths() {
    let mut out = vec![0u8; 32];
    hmac::pbkdf2_sha256(&det(0, b"pp"), &det(8, b"ps"), 1, &mut out);
    assert_eq!(out, unhex("57067da36c8ac6b755cb747a4d550c22e73b8bea84523e916e4819ab3fbd23fa"));
    let mut out = vec![0u8; 48];
    hmac::pbkdf2_sha256(&det(10, b"pp"), &det(16, b"ps"), 3, &mut out);
    assert_eq!(out, unhex("8312e375b93f4e5651cdbcb730e7b3e532bc1ef4a23476facdfcc657352870f77bb445d407d7287bd5ae0ca1c7bedfe7"));
    let mut out = vec![0u8; 100];
    hmac::pbkdf2_sha256(&det(70, b"pp"), &det(20, b"ps"), 1000, &mut out);
    assert_eq!(out, unhex("c89a5f0a2a7ad9014ffb4b596d4c0427b6cb2d8cdf6a9ee5d719126691c4511ce4bf8fa24f0839fd0a0700e5c5098b0c62e11229af7047cbbbb9318109bb0298571a4dd429631bfd0e14ebe95daddb7fa77025cfe8630061037a5e698efd6a792508311a"));
}

#[test]
fn poly1305_modular_edges() {
    assert_eq!(poly1305::poly1305(&unhex("0000000000000000000000000000000000000000000000000000000000000000").try_into().unwrap(), &unhex("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")).to_vec(), unhex("00000000000000000000000000000000"));
    assert_eq!(poly1305::poly1305(&unhex("0200000000000000000000000000000000000000000000000000000000000000").try_into().unwrap(), &unhex("ffffffffffffffffffffffffffffffff")).to_vec(), unhex("03000000000000000000000000000000"));
    assert_eq!(poly1305::poly1305(&unhex("02000000000000000000000000000000ffffffffffffffffffffffffffffffff").try_into().unwrap(), &unhex("02000000000000000000000000000000")).to_vec(), unhex("03000000000000000000000000000000"));
    assert_eq!(poly1305::poly1305(&unhex("0100000000000000000000000000000000000000000000000000000000000000").try_into().unwrap(), &unhex("fffffffffffffffffffffffffffffffff0ffffffffffffffffffffffffffffff11000000000000000000000000000000")).to_vec(), unhex("05000000000000000000000000000000"));
    assert_eq!(poly1305::poly1305(&unhex("0100000000000000000000000000000000000000000000000000000000000000").try_into().unwrap(), &unhex("fffffffffffffffffffffffffffffffffbfefefefefefefefefefefefefefefe01010101010101010101010101010101")).to_vec(), unhex("00000000000000000000000000000000"));
    assert_eq!(poly1305::poly1305(&unhex("0200000000000000000000000000000000000000000000000000000000000000").try_into().unwrap(), &unhex("fdffffffffffffffffffffffffffffff")).to_vec(), unhex("faffffffffffffffffffffffffffffff"));
    assert_eq!(poly1305::poly1305(&unhex("0100000000000000040000000000000000000000000000000000000000000000").try_into().unwrap(), &unhex("e33594d7505e43b900000000000000003394d7505e4379cd01000000000000000000000000000000000000000000000001000000000000000000000000000000")).to_vec(), unhex("14000000000000005500000000000000"));
    assert_eq!(poly1305::poly1305(&unhex("0100000000000000040000000000000000000000000000000000000000000000").try_into().unwrap(), &unhex("e33594d7505e43b900000000000000003394d7505e4379cd010000000000000000000000000000000000000000000000")).to_vec(), unhex("13000000000000000000000000000000"));
    assert_eq!(poly1305::poly1305(&det(32, b"pk0").try_into().unwrap(), &det(0, b"pm")).to_vec(), unhex("13a7c9c6d3cb720a683dc29e3252e2ea"), "longueur 0");
    assert_eq!(poly1305::poly1305(&det(32, b"pk1").try_into().unwrap(), &det(1, b"pm")).to_vec(), unhex("f7a1ddd5f291bece1a1667b961742498"), "longueur 1");
    assert_eq!(poly1305::poly1305(&det(32, b"pk15").try_into().unwrap(), &det(15, b"pm")).to_vec(), unhex("51ae142444d20f30d9d36a5e8e33c5d6"), "longueur 15");
    assert_eq!(poly1305::poly1305(&det(32, b"pk16").try_into().unwrap(), &det(16, b"pm")).to_vec(), unhex("be6dc1587219181874ff13603bb3f056"), "longueur 16");
    assert_eq!(poly1305::poly1305(&det(32, b"pk17").try_into().unwrap(), &det(17, b"pm")).to_vec(), unhex("13b9e6e28fcee3c322c67f5e9d0e5516"), "longueur 17");
    assert_eq!(poly1305::poly1305(&det(32, b"pk31").try_into().unwrap(), &det(31, b"pm")).to_vec(), unhex("5b9eb80d00f8d8240fbd0a0921a02e12"), "longueur 31");
    assert_eq!(poly1305::poly1305(&det(32, b"pk32").try_into().unwrap(), &det(32, b"pm")).to_vec(), unhex("09a1f23b896c807735539a4848af3c6b"), "longueur 32");
    assert_eq!(poly1305::poly1305(&det(32, b"pk33").try_into().unwrap(), &det(33, b"pm")).to_vec(), unhex("1256cacc10bdc11977c0daf2c22eda23"), "longueur 33");
    assert_eq!(poly1305::poly1305(&det(32, b"pk100").try_into().unwrap(), &det(100, b"pm")).to_vec(), unhex("24b9e5e8f357a762ab7d5c3d24bb38d7"), "longueur 100");
}

#[test]
fn chacha20_counters_and_lengths() {
    let mut data = vec![0u8; 1];
    chacha20::apply(&det(32, b"ck").try_into().unwrap(), 0x0, &det(12, b"cn").try_into().unwrap(), &mut data);
    assert_eq!(data, unhex("45"), "compteur 0, 1 octets");
    let mut data = vec![0u8; 64];
    chacha20::apply(&det(32, b"ck").try_into().unwrap(), 0x1, &det(12, b"cn").try_into().unwrap(), &mut data);
    assert_eq!(data, unhex("69ad685cd81d937bc5f0c296e5d269ded62a2524044da840594c7566d6d2e8bd185b09d55f28f50327634bd8b70f1a6c419ae34a391e6e082efeecf70b073263"), "compteur 1, 64 octets");
    let mut data = vec![0u8; 65];
    chacha20::apply(&det(32, b"ck").try_into().unwrap(), 0x1, &det(12, b"cn").try_into().unwrap(), &mut data);
    assert_eq!(data, unhex("69ad685cd81d937bc5f0c296e5d269ded62a2524044da840594c7566d6d2e8bd185b09d55f28f50327634bd8b70f1a6c419ae34a391e6e082efeecf70b07326393"), "compteur 1, 65 octets");
    let mut data = vec![0u8; 200];
    chacha20::apply(&det(32, b"ck").try_into().unwrap(), 0x7, &det(12, b"cn").try_into().unwrap(), &mut data);
    assert_eq!(data, unhex("995a5f617238e7b3cb22e52c177fcf70f76b4c4dbc2a27e1d50b3dc1e08ad377cb57930f901239b308778912041fa713971c2c9a044ff3ab3645827a2e6ac8329839ae8c5d78a391f789a7460cfb1b62198d7c362089007605c556ca05d771305a9b85c13df37d9560433379e86559d81b0010db97b9670d3c9661fd91162df5411ba6f5371cd6d049ba9179470bd6ea41d59355334f1b108d4cf315aadc59a0a3c8d1e1056a04ef4c6c71f297e07e80e35388ba2a696a495c897a5897dd56a61d2d4af0284b52cf"), "compteur 7, 200 octets");
    let mut data = vec![0u8; 64];
    chacha20::apply(&det(32, b"ck").try_into().unwrap(), 0xffffffff, &det(12, b"cn").try_into().unwrap(), &mut data);
    assert_eq!(data, unhex("e9547a4d14be169a5ce3971dc753de4c8aebd45dad364346e037594b238db15f4d7582d8efb9ea7e997860a3e7271b5bd2563dfbb60bb77adb20015c85833f12"), "compteur 4294967295, 64 octets");
}

#[test]
fn aead_lengths_match_the_reference() {
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(0, b"ad"), &det(0, b"ap"));
    assert_eq!(sealed, unhex("be1beded633bd2b77aa038fc3592fd62"), "0 octets, aad 0");
    assert_eq!(aead::decrypt(&key, &nonce, &det(0, b"ad"), &sealed), Some(det(0, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(5, b"ad"), &det(0, b"ap"));
    assert_eq!(sealed, unhex("1210b95b7ec5d854f49816b546ff6d35"), "0 octets, aad 5");
    assert_eq!(aead::decrypt(&key, &nonce, &det(5, b"ad"), &sealed), Some(det(0, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(0, b"ad"), &det(1, b"ap"));
    assert_eq!(sealed, unhex("6ed2c600a61e383b62fc0f6ca415865824"), "1 octets, aad 0");
    assert_eq!(aead::decrypt(&key, &nonce, &det(0, b"ad"), &sealed), Some(det(1, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(3, b"ad"), &det(15, b"ap"));
    assert_eq!(sealed, unhex("6e3eea3a2b9265b551c0b4c235791023a3cf1df4c3a22fcb118b1cb84b4d69"), "15 octets, aad 3");
    assert_eq!(aead::decrypt(&key, &nonce, &det(3, b"ad"), &sealed), Some(det(15, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(16, b"ad"), &det(16, b"ap"));
    assert_eq!(sealed, unhex("6e3eea3a2b9265b551c0b4c23579100a0b4a8ee89882c4f3f7d42b17dfa6d9ff"), "16 octets, aad 16");
    assert_eq!(aead::decrypt(&key, &nonce, &det(16, b"ad"), &sealed), Some(det(16, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(17, b"ad"), &det(17, b"ap"));
    assert_eq!(sealed, unhex("6e3eea3a2b9265b551c0b4c23579100a66448c8b6303cd69320f57ad7be437aeba"), "17 octets, aad 17");
    assert_eq!(aead::decrypt(&key, &nonce, &det(17, b"ad"), &sealed), Some(det(17, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(0, b"ad"), &det(63, b"ap"));
    assert_eq!(sealed, unhex("6e3eea3a2b9265b551c0b4c23579100a668778a0315f8dee5f02fba1bd8a6b0f216160dcba1a576d9aea7ff24e09b28cb28f895edff16dd35e49c700af81fc40255fb6dda1c9d19ebfcbc73179df55"), "63 octets, aad 0");
    assert_eq!(aead::decrypt(&key, &nonce, &det(0, b"ad"), &sealed), Some(det(63, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(1, b"ad"), &det(64, b"ap"));
    assert_eq!(sealed, unhex("6e3eea3a2b9265b551c0b4c23579100a668778a0315f8dee5f02fba1bd8a6b0f216160dcba1a576d9aea7ff24e09b28cb28f895edff16dd35e49c700af81fc792a800df59c2908fe80de2541c8737661"), "64 octets, aad 1");
    assert_eq!(aead::decrypt(&key, &nonce, &det(1, b"ad"), &sealed), Some(det(64, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(100, b"ad"), &det(65, b"ap"));
    assert_eq!(sealed, unhex("6e3eea3a2b9265b551c0b4c23579100a668778a0315f8dee5f02fba1bd8a6b0f216160dcba1a576d9aea7ff24e09b28cb28f895edff16dd35e49c700af81fc79ac98d53ade8ef8f40d1a428d7e6d00db0d"), "65 octets, aad 100");
    assert_eq!(aead::decrypt(&key, &nonce, &det(100, b"ad"), &sealed), Some(det(65, b"ap")));
    let (key, nonce): ([u8; 32], [u8; 12]) = (det(32, b"ak").try_into().unwrap(), det(12, b"an").try_into().unwrap());
    let sealed = aead::encrypt(&key, &nonce, &det(7, b"ad"), &det(1000, b"ap"));
    assert_eq!(sealed, unhex("6e3eea3a2b9265b551c0b4c23579100a668778a0315f8dee5f02fba1bd8a6b0f216160dcba1a576d9aea7ff24e09b28cb28f895edff16dd35e49c700af81fc79ac406407dea0b7d28d5e9579719c3780d9ebad52c00ae26f5acc42f37ae8727729ba4fa5ec6e7ab565ef08976645528e077c2db5fdeb45536d1abe314c11f7c1201d146fd13e9361fcf1fde1a3f34b2350f1e58b12298a5201e58f017c08ff62ffabe7003875e7af64acd243b6c9673fd8f132f15d6a3049c44c2f21b7c54e274263dab496641aa68f7556e9c79e7c515d77589319fd22dc5ea7387e2b4ce994a999a94c552253820cdb11960249c7ae18956eb5237943338fd2b6b6d18c736db754bec1f7f7dd86b21f4855db5ba23476cbc63771f5373a889fde156afe79f63f26585eac56027b714f5dd7826437bfc35d10183776e2fd3c90b939a127a869992610dd2868b0170f7cc07d6bd4d7aac7cc23cc1e1946fdba4b09b4d4276003a6a9f5e97df9df3f8ca0beb4643cf31a6a5c9c5f0c4acf254479adb2a8f852b1b5a4e9434bef4c6bd15975b889f0aa89992a4e7a6749028e06d2cc4cf9cfc720dfea7e9c99e9303b85b0e5b3b9fa07490adec79b4b14a9b898d4ccd6b79433d42caa95f6506847eef6a630cba3629e0db7ed60d9bd7f3f0b4da6f4f8791ef5e1cd5976f084f8796cd38fb151e3beb7964eba688c750b69b2046e186edcd052dac6ed358102251fff89f61e5631f2c7fd1da242986b184f87a967cbca4f9cd558e316da6e2754c65c0b71154f37606a9b17b44a7676f663017292eff9e76512c59dea2d0ec079ea18e31ad1c5204096109cdf8fbf92e31a815c5bba002b98d216b84fa7904fbe22feb0f8be42c64b8c846dc79689ea129a1ef927cda74032352674f468c1c69ca8fa8e45fa0e7b95620c22fa9c2097b84f72dd6597496468adf91fa58eaa8ffeca53ef530ba1399c7d6d64ee2ac8921aff097760e7919dbdfd91475b1fce207c9e6b13060fffe3fb82ea544253ddec11385a94b96bcbc26a7dd2417d294c0e664376804da5940829529c67bbc55188221a1827ae05bbb94f724e90ff609a4906ec21d89e09482100873c00a43e5f14bd0a03d8e62e02bd8c4bc81676232a1e523f3b9122fe312b41605f228fedead71baf6cda98f0155e2da1f1ce35eb13222c21893237df5e3fd4e47b205742e2087819325cc10d7dff317c031d5abb82647bea48bce0d3508a807bab571a8d2855f54521abf685b6dcc47ee780a52e4a6ffa842b8569cf8d3e48fc792f7ad41b60c7ef7df6b000e5b26eed6ef469005e01d1b83cb59033c83782220fb85066e5ba258362c11cf5ca119403c777050aecb974fd61d28cc9feb6414a4fd97dbf16ff170a288eb05655f8579044012f5a90602288372b58d92751228a684a1444cd242109e0"), "1000 octets, aad 7");
    assert_eq!(aead::decrypt(&key, &nonce, &det(7, b"ad"), &sealed), Some(det(1000, b"ap")));
    // Tronque, ou etiquette modifiee : refuse.
    let sealed = aead::encrypt(&[1; 32], &[2; 12], b"", b"message");
    assert_eq!(aead::decrypt(&[1; 32], &[2; 12], b"", &sealed[..sealed.len() - 1]), None);
    assert_eq!(aead::decrypt(&[1; 32], &[2; 12], b"", &sealed[..5]), None);
    let mut bad = sealed.clone();
    *bad.last_mut().unwrap() ^= 0x80;
    assert_eq!(aead::decrypt(&[1; 32], &[2; 12], b"", &bad), None);
}
