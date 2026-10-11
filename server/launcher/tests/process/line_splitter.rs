use bvc_launcher::process::LineSplitter;

#[test]
fn complete_lines_are_returned_with_their_newline() {
    let mut splitter = LineSplitter::new();

    assert_eq!(
        splitter.push(b"list\nstop\n"),
        vec![b"list\n".to_vec(), b"stop\n".to_vec()]
    );
}

#[test]
fn a_line_split_across_reads_is_joined() {
    let mut splitter = LineSplitter::new();

    assert!(splitter.push(b"bvc admin gr").is_empty());
    assert_eq!(
        splitter.push(b"ant Alaydriem\n"),
        vec![b"bvc admin grant Alaydriem\n".to_vec()]
    );
}

#[test]
fn a_trailing_partial_line_is_returned_at_the_end() {
    let mut splitter = LineSplitter::new();
    splitter.push(b"say hi");

    assert_eq!(splitter.finish(), Some(b"say hi".to_vec()));
    assert_eq!(splitter.finish(), None);
}
