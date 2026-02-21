# IPC Schema

Schema version: `1`

## Browser -> Content

| Message | Fields |
| --- | --- |
| `LoadDocument` | `request_id: u64`, `url: string`, `html: string`, `viewport_width: u32`, `viewport_height: u32` |
| `Tick` | `frame_index: u64` |
| `Shutdown` | (none) |

## Content -> Browser

| Message | Fields |
| --- | --- |
| `DocumentReady` | `request_id: u64`, `command_count: u32` |
| `Log` | `level: u8`, `message: string` |
| `AckShutdown` | (none) |
