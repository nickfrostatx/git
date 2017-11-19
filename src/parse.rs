use std::io::Read;
use types::GitResult;

// Read from a reader up to, and not including, some end character
pub fn read_until(reader: &mut Read, end: u8) -> GitResult<Vec<u8>> {
    let mut content = vec![];
    let mut buf = vec![0];

    loop {
        reader.read_exact(&mut buf)?;
        if buf[0] == end {
            break;
        }
        content.push(buf[0]);
    }

    Ok(content)
}
