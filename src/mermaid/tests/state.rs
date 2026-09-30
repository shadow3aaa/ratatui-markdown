use super::common::*;

static SIMPLE_STATE: &str = "stateDiagram-v2
    [*] --> Idle
    Idle --> Running
    Running --> Idle";

static SIMPLE_STATE_EXPECTED: &str = "
                             ╭────▲────╮    ╭────╮
                             │ Running │    │ ●  │
                             ╰─────────╯    ╰────╯
                                  │            │
                                  ├─────┬──────┘
                                  │     ▼
                                  │ ╭──────╮
                                  │ │ Idle │
                                  │ ╰──────╯";

static THREE_STATES: &str = "stateDiagram-v2
    [*] --> A
    A --> B
    B --> C";

static THREE_STATES_EXPECTED: &str = "
                                     ╭────╮
                                     │ ●  │
                                     ╰────╯
                                        │
                                        │
                                        ▼
                                     ╭────╮
                                     │ A  │
                                     ╰────╯
                                        │
                                        │
                                        ▼";

static END_STATE: &str = "stateDiagram-v2
    [*] --> S1
    S1 --> S2
    S2 --> [*]";

static END_STATE_EXPECTED: &str = "
                                     \u{256d}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}
                                     \u{2502} \u{25cf}  \u{2502}
                                     \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f}
                                        \u{2502}
                                        \u{2502}
                                        \u{25bc}
                                     \u{256d}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}
                                     \u{2502} S1 \u{2502}
                                     \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f}
                                        \u{2502}
                                        \u{2502}
                                        \u{25bc}
                                     \u{256d}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}
                                     \u{2502} S2 \u{2502}
                                     \u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f}
                                        \u{2502}
                                        \u{2502}
                                        \u{25bc}
                                     \u{256d}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}
                                     \u{2502} \u{25cf}  \u{2502}";

#[test]
fn simple_state() {
    let buf = render_to_buffer(SIMPLE_STATE, 80, 10);
    assert_buffer_eq(&buf, SIMPLE_STATE_EXPECTED);
}

#[test]
fn three_states() {
    let buf = render_to_buffer(THREE_STATES, 80, 12);
    assert_buffer_eq(&buf, THREE_STATES_EXPECTED);
}

#[test]
fn end_state() {
    let buf = render_to_buffer(END_STATE, 80, 20);
    assert_buffer_eq(&buf, END_STATE_EXPECTED);
}
