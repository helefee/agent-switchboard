use super::*;
use crate::extensions::checks::protocol::is_response;
use serde_json::json;

#[test]
fn fragmented_stream_preserves_multiline_events_and_matches_the_requested_response() {
    let mut decoder = SseDecoder::default();
    let chunks: &[&[u8]] = &[
        b": keepalive\r\n\r\ndata:\r\n\r\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\"}\n\n",
        b"data: {\"jsonrpc\":\"2.0\",\"id\":99,\"result\":{}}\n\ndata: {\"jsonrpc\":\"2.0\",\r",
        b"\ndata: \"id\":7,\"result\":{\"tools\":[]}}\r\n\r",
        b"\n: stream remains open\r\n",
    ];
    let messages = chunks.iter().flat_map(|bytes| decoder.push(bytes).unwrap()).collect::<Vec<_>>();
    let response = messages.iter().find(|value| is_response(value, &json!(7))).unwrap();
    assert_eq!(response["result"]["tools"], json!([]));
    assert_eq!(messages.len(), 3);
}

#[test]
fn accepts_cr_lines_and_a_utf8_bom_without_requiring_eof() {
    let mut decoder = SseDecoder::default();
    let messages = decoder.push("\u{feff}data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\r\r".as_bytes()).unwrap();
    assert!(is_response(&messages[0], &json!(1)));
}

#[test]
fn stream_output_is_bounded_even_without_a_complete_line() {
    let mut decoder = SseDecoder::default();
    let error = decoder.push(&vec![b'x'; MAX_OUTPUT_BYTES + 1]).unwrap_err();
    assert_eq!(error.kind, crate::extensions::checks::CheckErrorKind::OutputLimit);
}
