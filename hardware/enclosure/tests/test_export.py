import struct
import zipfile

import pytest
from build123d import import_step
from helpers import inside

from drezimo_case import __main__ as cli
from drezimo_case.export import export_part, to_print


@pytest.fixture(scope="module")
def exported(tmp_path_factory, body, inlay):
    out = tmp_path_factory.mktemp("print")
    return out, export_part("coupon-a", body, inlay, out)


def test_print_orientation_lays_the_deck_on_the_plate(body, p):
    flat = to_print(body, p.deck_top)
    box = flat.bounding_box()
    assert (box.min.Z, box.max.Z) == pytest.approx((0.0, p.deck_top + 1.0), abs=1e-6)
    assert flat.volume == pytest.approx(body.volume)
    # Only z flips, x and y stay: that is what turns the left-handed case frame into the real part.
    assert inside(flat, 4.9, 47.0, p.deck_top - 0.05)
    assert not inside(flat, 4.9, 47.0, p.deck_top + 0.05)


def test_step_holds_the_two_labelled_parts(exported, body, inlay):
    out, paths = exported
    assert {path.suffix for path in paths} == {".step", ".3mf", ".png"}
    parts = {part.label: part for part in import_step(out / "coupon-a.step").children}
    assert set(parts) == {"body", "inlay"}
    assert parts["body"].volume == pytest.approx(body.volume, rel=1e-3)
    assert parts["inlay"].volume == pytest.approx(inlay.volume, rel=1e-3)
    assert parts["body"].bounding_box().min.Z == pytest.approx(0.0, abs=1e-6)


def test_3mf_holds_one_object_per_part(exported):
    out, _ = exported
    model = zipfile.ZipFile(out / "coupon-a.3mf").read("3D/3dmodel.model").decode()
    assert model.count("<item ") == 2


def test_preview_is_a_1600_by_1200_picture(exported):
    out, _ = exported
    header = (out / "coupon-a.png").read_bytes()[:24]
    assert header[:8] == b"\x89PNG\r\n\x1a\n"
    assert struct.unpack(">II", header[16:24]) == (1600, 1200)


def test_command_writes_the_files_and_names_the_estimates(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    cli.main(["coupon-a"])
    assert {path.name for path in tmp_path.iterdir()} >= {"coupon-a.step", "coupon-a.3mf", "coupon-a.png"}
    assert "M2b" in capsys.readouterr().out
