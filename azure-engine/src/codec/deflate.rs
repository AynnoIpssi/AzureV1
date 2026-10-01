// Decompresseur DEFLATE (RFC 1951) ecrit de zero, sans aucune dependance -
// c'est le coeur algorithmique dont `codec::zlib` (l'enveloppe RFC 1950
// utilisee par PNG) a besoin. Base directement sur l'algorithme de
// reference documente dans puff.c (la reference minimale de zlib pour
// DEFLATE) : construction canonique de Huffman par comptages/offsets,
// decodage bit-a-bit sans table de lookup rapide - plus simple a ecrire et
// a verifier correct qu'une implementation optimisee, largement suffisant
// pour decoder des images de la taille d'une icone/un logo, pas un flux
// video.

/// Lit les bits d'un flux d'octets dans l'ordre DEFLATE : LSB en premier a
/// l'interieur de chaque octet (a l'inverse d'un code de Huffman, qui lui
/// s'accumule MSB en premier au fil des bits lus - voir `HuffmanTable::decode`).
struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, byte_pos: 0, bit_pos: 0 }
    }

    fn read_bit(&mut self) -> Result<u32, String> {
        if self.byte_pos >= self.data.len() {
            return Err("DEFLATE: fin de flux inattendue".to_string());
        }
        let bit = (self.data[self.byte_pos] >> self.bit_pos) & 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Ok(bit as u32)
    }

    // Assemble `n` bits en un entier LSB-en-premier (convention DEFLATE
    // pour tout ce qui n'est PAS un code de Huffman : bits d'extra-longueur/
    // distance, en-tetes de bloc...) - a ne pas confondre avec
    // `HuffmanTable::decode`, qui accumule MSB en premier.
    fn read_bits(&mut self, n: u32) -> Result<u32, String> {
        let mut value = 0u32;
        for i in 0..n {
            value |= self.read_bit()? << i;
        }
        Ok(value)
    }

    fn align_to_byte(&mut self) {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        if self.byte_pos >= self.data.len() {
            return Err("DEFLATE: fin de flux inattendue".to_string());
        }
        let b = self.data[self.byte_pos];
        self.byte_pos += 1;
        Ok(b)
    }
}

const MAX_BITS: usize = 15;

/// Une table de decodage de Huffman canonique, construite depuis un tableau
/// de longueurs de code (index = symbole, valeur = longueur en bits, 0 =
/// symbole absent de cet arbre) - construction et decodage tous deux
/// repris de l'algorithme de reference `construct()`/`decode()` de puff.c.
struct HuffmanTable {
    // counts[len] = nombre de codes de longueur `len` (1..=15) ; counts[0]
    // toujours 0 (un code de longueur 0 = symbole absent, pas un vrai code).
    counts: [u16; MAX_BITS + 1],
    // Les symboles, groupes par longueur croissante puis par ordre de
    // symbole croissant au sein d'une meme longueur (ordre canonique) -
    // c'est cet ordre qui permet a `decode` de retrouver le symbole a
    // partir de sa position dans sa tranche de longueur.
    symbols: Vec<u16>,
}

impl HuffmanTable {
    fn build(lengths: &[u8]) -> HuffmanTable {
        let mut counts = [0u16; MAX_BITS + 1];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0;

        let mut offsets = [0u16; MAX_BITS + 1];
        for len in 1..=MAX_BITS {
            offsets[len] = offsets[len - 1] + counts[len - 1];
        }

        let total: usize = counts.iter().map(|&c| c as usize).sum();
        let mut symbols = vec![0u16; total];
        let mut next = offsets;
        for (sym, &len) in lengths.iter().enumerate() {
            if len == 0 {
                continue;
            }
            let l = len as usize;
            symbols[next[l] as usize] = sym as u16;
            next[l] += 1;
        }

        HuffmanTable { counts, symbols }
    }

    fn decode(&self, reader: &mut BitReader) -> Result<u16, String> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;
        for len in 1..=MAX_BITS {
            code |= reader.read_bit()? as i32;
            let count = self.counts[len] as i32;
            if code - first < count {
                return Ok(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err("DEFLATE: code de Huffman invalide".to_string())
    }
}

// Table 3.2.5 de la RFC 1951 : base et bits supplementaires pour les codes
// de longueur 257..285.
const LENGTH_BASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LENGTH_EXTRA: [u32; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
// Meme table pour les codes de distance 0..29.
const DIST_BASE: [u16; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DIST_EXTRA: [u32; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
// Ordre dans lequel les longueurs de l'alphabet "longueurs de code" (19
// symboles) sont transmises pour un bloc a Huffman dynamique - RFC 1951
// section 3.2.7, un ordre fixe qui n'a rien d'intuitif (les longueurs les
// plus frequemment utiles en pratique sont placees en premier).
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

fn fixed_length_table() -> HuffmanTable {
    let mut lengths = [0u8; 288];
    for l in lengths.iter_mut().take(144) {
        *l = 8;
    }
    for l in lengths.iter_mut().take(256).skip(144) {
        *l = 9;
    }
    for l in lengths.iter_mut().take(280).skip(256) {
        *l = 7;
    }
    for l in lengths.iter_mut().take(288).skip(280) {
        *l = 8;
    }
    HuffmanTable::build(&lengths)
}

fn fixed_distance_table() -> HuffmanTable {
    HuffmanTable::build(&[5u8; 30])
}

// Decode les longueurs de code litteral/longueur ET distance d'un bloc a
// Huffman dynamique (RFC 1951 section 3.2.7) : d'abord la table (elle-meme
// Huffman-codee) des longueurs de l'alphabet "longueurs de code", puis les
// `hlit + hdist` longueurs reelles a travers cette table, avec 3 codes de
// repetition (16/17/18) pour ne pas avoir a coder chaque longueur nulle
// individuellement.
fn read_dynamic_tables(reader: &mut BitReader) -> Result<(HuffmanTable, HuffmanTable), String> {
    let hlit = reader.read_bits(5)? as usize + 257;
    let hdist = reader.read_bits(5)? as usize + 1;
    let hclen = reader.read_bits(4)? as usize + 4;

    let mut code_length_lengths = [0u8; 19];
    for &order_idx in CODE_LENGTH_ORDER.iter().take(hclen) {
        code_length_lengths[order_idx] = reader.read_bits(3)? as u8;
    }
    let code_length_table = HuffmanTable::build(&code_length_lengths);

    let mut lengths: Vec<u8> = Vec::with_capacity(hlit + hdist);
    while lengths.len() < hlit + hdist {
        let symbol = code_length_table.decode(reader)?;
        match symbol {
            0..=15 => lengths.push(symbol as u8),
            16 => {
                let prev = *lengths.last().ok_or("DEFLATE: code 16 sans longueur precedente a repeter")?;
                let repeat = reader.read_bits(2)? + 3;
                for _ in 0..repeat {
                    lengths.push(prev);
                }
            }
            17 => {
                let repeat = reader.read_bits(3)? + 3;
                lengths.resize(lengths.len() + repeat as usize, 0);
            }
            18 => {
                let repeat = reader.read_bits(7)? + 11;
                lengths.resize(lengths.len() + repeat as usize, 0);
            }
            _ => return Err("DEFLATE: symbole invalide dans l'alphabet des longueurs de code".to_string()),
        }
    }
    if lengths.len() != hlit + hdist {
        return Err("DEFLATE: nombre de longueurs de code decodees incoherent avec hlit/hdist".to_string());
    }

    let lit_table = HuffmanTable::build(&lengths[..hlit]);
    let dist_table = HuffmanTable::build(&lengths[hlit..]);
    Ok((lit_table, dist_table))
}

// Decode un bloc compresse (BTYPE 01 ou 10, qui ne different que par la
// table de Huffman utilisee) directement dans `output` - litteraux copies
// tels quels, couples longueur/distance re-copies depuis une position deja
// ecrite de `output` (LZ77 : un chevauchement, distance < longueur, est
// volontaire et doit re-lire les octets tout juste ecrits, d'ou la copie
// octet par octet plutot qu'un `copy_within`).
fn inflate_block(reader: &mut BitReader, lit_table: &HuffmanTable, dist_table: &HuffmanTable, output: &mut Vec<u8>) -> Result<(), String> {
    loop {
        let symbol = lit_table.decode(reader)?;
        if symbol < 256 {
            output.push(symbol as u8);
        } else if symbol == 256 {
            return Ok(());
        } else {
            let idx = (symbol - 257) as usize;
            if idx >= LENGTH_BASE.len() {
                return Err("DEFLATE: code de longueur invalide".to_string());
            }
            let length = LENGTH_BASE[idx] as usize + reader.read_bits(LENGTH_EXTRA[idx])? as usize;

            let dist_symbol = dist_table.decode(reader)? as usize;
            if dist_symbol >= DIST_BASE.len() {
                return Err("DEFLATE: code de distance invalide".to_string());
            }
            let distance = DIST_BASE[dist_symbol] as usize + reader.read_bits(DIST_EXTRA[dist_symbol])? as usize;

            if distance > output.len() {
                return Err("DEFLATE: distance de retro-reference au-dela du debut du flux decompresse".to_string());
            }
            // Octet par octet : la copie peut chevaucher ce qu'elle ecrit
            // (distance < longueur), comme le veut DEFLATE.
            let start = output.len() - distance;
            for src in start..start + length {
                let byte = output[src];
                output.push(byte);
            }
        }
    }
}

/// Decompresse un flux DEFLATE brut (RFC 1951, sans l'enveloppe zlib - voir
/// `codec::zlib::decompress` pour ca) en un tampon d'octets.
pub fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = BitReader::new(data);
    let mut output = Vec::new();

    loop {
        let bfinal = reader.read_bit()?;
        let btype = reader.read_bits(2)?;

        match btype {
            0 => {
                // Bloc non compresse : LEN/NLEN sont alignes sur l'octet,
                // donc tout bit deja consomme dans l'octet courant (par
                // BFINAL/BTYPE) est jete avant de les lire.
                reader.align_to_byte();
                let len_lo = reader.read_byte()? as u16;
                let len_hi = reader.read_byte()? as u16;
                let len = len_lo | (len_hi << 8);
                let nlen_lo = reader.read_byte()? as u16;
                let nlen_hi = reader.read_byte()? as u16;
                let nlen = nlen_lo | (nlen_hi << 8);
                if len != !nlen {
                    return Err("DEFLATE: LEN/NLEN incoherents dans un bloc non compresse".to_string());
                }
                for _ in 0..len {
                    output.push(reader.read_byte()?);
                }
            }
            1 => {
                let lit = fixed_length_table();
                let dist = fixed_distance_table();
                inflate_block(&mut reader, &lit, &dist, &mut output)?;
            }
            2 => {
                let (lit, dist) = read_dynamic_tables(&mut reader)?;
                inflate_block(&mut reader, &lit, &dist, &mut output)?;
            }
            _ => return Err("DEFLATE: type de bloc reserve (invalide)".to_string()),
        }

        if bfinal == 1 {
            return Ok(output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Construit a la main un bloc DEFLATE non compresse (BTYPE=00) -
    // BFINAL=1, BTYPE=00 tiennent dans les 3 premiers bits du premier
    // octet (avec padding a zero jusqu'a l'octet), puis LEN/NLEN/donnees.
    fn stored_block(data: &[u8]) -> Vec<u8> {
        let mut out = vec![0b001u8]; // BFINAL=1 (bit 0), BTYPE=00 (bits 1-2), reste = padding
        let len = data.len() as u16;
        out.push((len & 0xFF) as u8);
        out.push((len >> 8) as u8);
        let nlen = !len;
        out.push((nlen & 0xFF) as u8);
        out.push((nlen >> 8) as u8);
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn stored_block_round_trips_raw_bytes() {
        let original = b"hello, azure!";
        let compressed = stored_block(original);
        let decoded = inflate(&compressed).expect("inflate a reussi");
        assert_eq!(decoded, original);
    }

    #[test]
    fn empty_stored_block_yields_empty_output() {
        let compressed = stored_block(&[]);
        let decoded = inflate(&compressed).expect("inflate a reussi");
        assert!(decoded.is_empty());
    }

    #[test]
    fn mismatched_len_and_nlen_is_an_error() {
        let mut compressed = stored_block(b"abc");
        compressed[3] ^= 0xFF; // corrompt NLEN
        assert!(inflate(&compressed).is_err());
    }

    #[test]
    fn huffman_table_decodes_every_symbol_it_was_built_from() {
        // 4 symboles de longueurs 1,2,3,3 (un arbre de Huffman canonique
        // valide) : code 0="0", code 10="1", code 110="2", code 111="3".
        let table = HuffmanTable::build(&[1, 2, 3, 3]);
        let bits = [0u8, 1, 0, 1, 1, 0, 1, 1, 1];
        // Empaquete `bits` (un bit par octet, pour la lisibilite du test)
        // dans un vrai flux d'octets LSB-en-premier.
        let mut packed = vec![0u8; 2];
        for (i, &b) in bits.iter().enumerate() {
            if b == 1 {
                packed[i / 8] |= 1 << (i % 8);
            }
        }
        let mut reader = BitReader::new(&packed);
        assert_eq!(table.decode(&mut reader).unwrap(), 0);
        assert_eq!(table.decode(&mut reader).unwrap(), 1);
        assert_eq!(table.decode(&mut reader).unwrap(), 2);
        assert_eq!(table.decode(&mut reader).unwrap(), 3);
    }
}
