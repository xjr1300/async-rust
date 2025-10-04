use std::io::{self, Cursor, Read, Write};

#[derive(Debug)]
pub struct Data {
    pub field1: u32,
    pub field2: u16,
    pub field3: String,
}

impl Data {
    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut bytes = vec![];
        bytes.write_all(&self.field1.to_ne_bytes())?;
        bytes.write_all(&self.field2.to_ne_bytes())?;
        let field3_len = self.field3.len() as u32;
        bytes.write_all(&field3_len.to_ne_bytes())?;
        bytes.extend_from_slice(self.field3.as_bytes());
        Ok(bytes)
    }

    pub fn deserialize(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let mut field1_bytes = [0u8; 4];
        let mut field2_bytes = [0u8; 2];
        cursor.read_exact(&mut field1_bytes)?;
        cursor.read_exact(&mut field2_bytes)?;
        let field1 = u32::from_ne_bytes(field1_bytes);
        let field2 = u16::from_ne_bytes(field2_bytes);

        let mut field3_len_bytes = [0u8; 4];
        cursor.read_exact(&mut field3_len_bytes)?;
        let field3_len = u32::from_ne_bytes(field3_len_bytes) as usize;
        let mut field3_bytes = vec![0u8; field3_len];
        let _ = cursor.read_exact(&mut field3_bytes);
        let field3 = String::from_utf8(field3_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 bytes"))?;
        Ok(Self {
            field1,
            field2,
            field3,
        })
    }
}
