use std::collections::VecDeque;

pub const IPC_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserToContent {
    LoadDocument {
        request_id: u64,
        url: String,
        html: String,
        viewport: Viewport,
    },
    Tick {
        frame_index: u64,
    },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentToBrowser {
    DocumentReady { request_id: u64, command_count: u32 },
    Log { level: u8, message: String },
    AckShutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    UnexpectedEof,
    InvalidTag(u8),
    InvalidUtf8,
}

#[derive(Debug, Default)]
pub struct InProcessTransport {
    to_content: VecDeque<Vec<u8>>,
    to_browser: VecDeque<Vec<u8>>,
}

impl InProcessTransport {
    pub fn send_to_content(&mut self, message: &BrowserToContent) {
        self.to_content
            .push_back(encode_browser_to_content(message));
    }

    pub fn recv_for_content(&mut self) -> Option<Result<BrowserToContent, CodecError>> {
        self.to_content
            .pop_front()
            .map(|payload| decode_browser_to_content(&payload))
    }

    pub fn send_to_browser(&mut self, message: &ContentToBrowser) {
        self.to_browser
            .push_back(encode_content_to_browser(message));
    }

    pub fn recv_for_browser(&mut self) -> Option<Result<ContentToBrowser, CodecError>> {
        self.to_browser
            .pop_front()
            .map(|payload| decode_content_to_browser(&payload))
    }
}

pub fn encode_browser_to_content(message: &BrowserToContent) -> Vec<u8> {
    let mut out = Vec::new();
    write_u32(&mut out, IPC_SCHEMA_VERSION);

    match message {
        BrowserToContent::LoadDocument {
            request_id,
            url,
            html,
            viewport,
        } => {
            write_u8(&mut out, 1);
            write_u64(&mut out, *request_id);
            write_string(&mut out, url);
            write_string(&mut out, html);
            write_u32(&mut out, viewport.width);
            write_u32(&mut out, viewport.height);
        }
        BrowserToContent::Tick { frame_index } => {
            write_u8(&mut out, 2);
            write_u64(&mut out, *frame_index);
        }
        BrowserToContent::Shutdown => {
            write_u8(&mut out, 3);
        }
    }

    out
}

pub fn decode_browser_to_content(bytes: &[u8]) -> Result<BrowserToContent, CodecError> {
    let mut cursor = Cursor::new(bytes);
    let _version = cursor.read_u32()?;
    let tag = cursor.read_u8()?;

    match tag {
        1 => {
            let request_id = cursor.read_u64()?;
            let url = cursor.read_string()?;
            let html = cursor.read_string()?;
            let width = cursor.read_u32()?;
            let height = cursor.read_u32()?;
            Ok(BrowserToContent::LoadDocument {
                request_id,
                url,
                html,
                viewport: Viewport { width, height },
            })
        }
        2 => {
            let frame_index = cursor.read_u64()?;
            Ok(BrowserToContent::Tick { frame_index })
        }
        3 => Ok(BrowserToContent::Shutdown),
        _ => Err(CodecError::InvalidTag(tag)),
    }
}

pub fn encode_content_to_browser(message: &ContentToBrowser) -> Vec<u8> {
    let mut out = Vec::new();
    write_u32(&mut out, IPC_SCHEMA_VERSION);

    match message {
        ContentToBrowser::DocumentReady {
            request_id,
            command_count,
        } => {
            write_u8(&mut out, 1);
            write_u64(&mut out, *request_id);
            write_u32(&mut out, *command_count);
        }
        ContentToBrowser::Log { level, message } => {
            write_u8(&mut out, 2);
            write_u8(&mut out, *level);
            write_string(&mut out, message);
        }
        ContentToBrowser::AckShutdown => {
            write_u8(&mut out, 3);
        }
    }

    out
}

pub fn decode_content_to_browser(bytes: &[u8]) -> Result<ContentToBrowser, CodecError> {
    let mut cursor = Cursor::new(bytes);
    let _version = cursor.read_u32()?;
    let tag = cursor.read_u8()?;

    match tag {
        1 => {
            let request_id = cursor.read_u64()?;
            let command_count = cursor.read_u32()?;
            Ok(ContentToBrowser::DocumentReady {
                request_id,
                command_count,
            })
        }
        2 => {
            let level = cursor.read_u8()?;
            let message = cursor.read_string()?;
            Ok(ContentToBrowser::Log { level, message })
        }
        3 => Ok(ContentToBrowser::AckShutdown),
        _ => Err(CodecError::InvalidTag(tag)),
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        if self.offset + len > self.bytes.len() {
            return Err(CodecError::UnexpectedEof);
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.bytes[start..self.offset])
    }

    fn read_u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, CodecError> {
        let mut buf = [0_u8; 4];
        buf.copy_from_slice(self.read_exact(4)?);
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&mut self) -> Result<u64, CodecError> {
        let mut buf = [0_u8; 8];
        buf.copy_from_slice(self.read_exact(8)?);
        Ok(u64::from_le_bytes(buf))
    }

    fn read_string(&mut self) -> Result<String, CodecError> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| CodecError::InvalidUtf8)
    }
}

fn write_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    write_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_to_content_roundtrip() {
        let message = BrowserToContent::LoadDocument {
            request_id: 44,
            url: "file:///test.html".to_string(),
            html: "<p>hello</p>".to_string(),
            viewport: Viewport {
                width: 800,
                height: 600,
            },
        };

        let encoded = encode_browser_to_content(&message);
        let decoded = decode_browser_to_content(&encoded).unwrap();
        assert_eq!(decoded, message);
    }

    #[test]
    fn content_to_browser_roundtrip() {
        let message = ContentToBrowser::Log {
            level: 2,
            message: "ready".to_string(),
        };

        let encoded = encode_content_to_browser(&message);
        let decoded = decode_content_to_browser(&encoded).unwrap();
        assert_eq!(decoded, message);
    }

    #[test]
    fn in_process_transport_smoke() {
        let mut transport = InProcessTransport::default();
        transport.send_to_content(&BrowserToContent::Tick { frame_index: 3 });

        let Some(message) = transport.recv_for_content() else {
            panic!("missing content message");
        };

        assert_eq!(message.unwrap(), BrowserToContent::Tick { frame_index: 3 });
    }
}
