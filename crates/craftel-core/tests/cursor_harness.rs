use craftel_core::{
    harness::{CursorHarness, NdjsonParser, STDERR_LIMIT, STDERR_TRUNCATED, append_bounded},
    runs::EventKind,
};
#[test]
fn fragmented_parser_and_exact_argv() {
    let mut p = NdjsonParser::default();
    assert!(p.push(b"{\"type\":\"assis").is_empty());
    let e = p.push(b"tant\",\"text\":\"ok\",\"new\":1}\nnope\n");
    assert_eq!(e[0].kind, EventKind::Assistant);
    assert_eq!(e[1].kind, EventKind::Unknown);
    let events = p.push(b"\n  {\"type\":\"tool_call\",\"call_id\":\"official\",\"tool_call\":{\"name\":\"shell\"}}  \n{\"type\":\"system\",\"model\":\"cursor-model\"}\n");
    assert_eq!(events[0].kind, EventKind::Unknown);
    assert_eq!(events[0].raw_json, "");
    assert_eq!(events[1].tool_call_id.as_deref(), Some("official"));
    assert_eq!(events[1].tool_name.as_deref(), Some("shell"));
    assert_eq!(
        events[1].raw_json,
        "  {\"type\":\"tool_call\",\"call_id\":\"official\",\"tool_call\":{\"name\":\"shell\"}}  "
    );
    assert_eq!(events[2].model.as_deref(), Some("cursor-model"));
    assert_eq!(
        CursorHarness::argv("$(touch nope)", Some("s")),
        vec![
            "-p",
            "--force",
            "--resume=s",
            "--output-format",
            "stream-json",
            "--stream-partial-output",
            "$(touch nope)"
        ]
    );
    let mut b = Vec::new();
    append_bounded(&mut b, &vec![b'x'; STDERR_LIMIT + 1]);
    assert!(String::from_utf8_lossy(&b).ends_with(STDERR_TRUNCATED));
}
