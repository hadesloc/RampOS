from rampos.cli.output import render_output


def test_output_json_is_default() -> None:
    assert render_output({"ok": True}).strip() == '{"ok": true}'


def test_output_json_compact_can_be_disabled() -> None:
    rendered = render_output({"ok": True}, compact=False)
    assert rendered.startswith("{\n")
