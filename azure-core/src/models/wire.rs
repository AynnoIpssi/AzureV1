// Format binaire maison, partage par les daemons d'Azure (stockage, provider,
// service) : fichiers et sockets. Entiers en little-endian (comme
// azure-rooter), octets et textes precedes de leur longueur en u32.

#[derive(Default)]
pub struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub fn new() -> Writer {
        Writer::default()
    }

    pub fn u8(mut self, value: u8) -> Writer {
        self.bytes.push(value);
        self
    }

    pub fn u32(mut self, value: u32) -> Writer {
        self.bytes.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn u64(mut self, value: u64) -> Writer {
        self.bytes.extend_from_slice(&value.to_le_bytes());
        self
    }

    pub fn bytes(self, value: &[u8]) -> Writer {
        let mut writer = self.u32(value.len() as u32);
        writer.bytes.extend_from_slice(value);
        writer
    }

    pub fn str(self, value: &str) -> Writer {
        self.bytes(value.as_bytes())
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Reader<'a> {
        Reader { data, pos: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(len).filter(|end| *end <= self.data.len()).ok_or("Donnee tronquee")?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u64(&mut self) -> Result<u64, String> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], String> {
        let len = self.u32()? as usize;
        self.take(len)
    }

    pub fn str(&mut self) -> Result<String, String> {
        String::from_utf8(self.bytes()?.to_vec()).map_err(|_| "Texte UTF-8 invalide".to_string())
    }

    /// Erreur s'il reste des octets non lus (donnee mal formee).
    pub fn finish(&self) -> Result<(), String> {
        if self.pos == self.data.len() { Ok(()) } else { Err("Octets en trop".to_string()) }
    }
}
