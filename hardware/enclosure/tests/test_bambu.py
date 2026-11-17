import json
import zipfile
from xml.etree import ElementTree

import pytest
from helpers import nominal

from drezimo_case import __main__ as cli
from drezimo_case import bambu

needs_bambu_studio = pytest.mark.skipif(not bambu.STUDIO.exists(), reason="Bambu Studio is not installed")

MODEL_SETTINGS = """<?xml version="1.0" encoding="UTF-8"?>
<config>
  <object id="3">
    <metadata key="name" value="label-gauge"/>
    <metadata key="extruder" value="1"/>
    <part id="1" subtype="normal_part">
      <metadata key="name" value="label-gauge"/>
      <mesh_stat face_count="12"/>
    </part>
    <part id="2" subtype="normal_part">
      <metadata key="name" value="label-gauge_2"/>
      <mesh_stat face_count="12"/>
    </part>
  </object>
  <plate>
    <metadata key="plater_id" value="1"/>
    <metadata key="locked" value="false"/>
  </plate>
</config>
"""

GCODE = """M620 S0A
T1000
; CHANGE_LAYER
; FEATURE: Prime tower
G1 X10 Y10 E.5
; FEATURE: Outer wall
G1 X20 Y10 E.5
G1 X20 Y20 E-.8
M620 S1A
T1
; FEATURE: Prime tower
G1 X10 Y12 E.5
; FEATURE: Outer wall
G1 X30 Y10 E.5
; CHANGE_LAYER
G1 X40 Y10 E.5
M620 S0A
T0
G1 X50 Y10 E.5
M620 S255A
"""


def write_preset(profiles, folder, name, **values):
    (profiles / folder).mkdir(parents=True, exist_ok=True)
    (profiles / folder / f"{name}.json").write_text(json.dumps({"name": name, **values}))


def exported_project(different=("", "", "", "")):
    settings = {"wall_generator": "classic", "enable_support": "0", "different_settings_to_system": list(different)}
    return {
        "Metadata/project_settings.config": json.dumps(settings).encode(),
        "Metadata/model_settings.config": MODEL_SETTINGS.encode(),
        "3D/3dmodel.model": b"<model/>",
    }


def decided(supported):
    files = bambu.decide(exported_project(), supported)
    settings = json.loads(files["Metadata/project_settings.config"])
    return files, settings, ElementTree.fromstring(files["Metadata/model_settings.config"])


def test_a_preset_is_resolved_through_its_parents_and_templates(tmp_path):
    write_preset(tmp_path, "process", "common", type="process", layer_height="0.3", line_width="0.4", wall_loops="2")
    write_preset(tmp_path, "process", "standard", inherits="common", line_width="0.42", top_shell_layers="5")
    write_preset(tmp_path, "filament", "template", instantiation="false", top_shell_layers="3", flow="0.95")
    write_preset(tmp_path, "process", "leaf", inherits="standard", include=["template"], layer_height="0.2", flow="0.98", setting_id="GP004")

    assert bambu.resolve_preset("leaf", tmp_path) == {
        "type": "process",
        "name": "leaf",
        "setting_id": "GP004",
        "instantiation": "false",
        "layer_height": "0.2",
        "line_width": "0.42",
        "wall_loops": "2",
        "top_shell_layers": "3",
        "flow": "0.98",
    }


def test_an_unknown_preset_is_named(tmp_path):
    with pytest.raises(FileNotFoundError, match="nowhere"):
        bambu.resolve_preset("nowhere", tmp_path)


def test_the_project_prints_black_and_white_on_the_textured_plate():
    _, settings, _ = decided(supported=False)
    assert settings["filament_colour"] == ["#1A1A1C", "#F5F2F0"]
    assert settings["curr_bed_type"] == "Textured PEI Plate"
    assert settings["flush_volumes_matrix"] == ["0", "800", "250", "0"]


def test_the_two_parts_are_named_and_given_their_filament():
    _, _, model = decided(supported=False)
    parts = [{m.get("key"): m.get("value") for m in part.findall("metadata")} for part in model.iter("part")]
    assert parts == [{"name": "body", "extruder": "1"}, {"name": "inlay", "extruder": "2"}]


def test_black_goes_down_first_on_the_first_layer():
    _, _, model = decided(supported=False)
    plate = {m.get("key"): m.get("value") for m in model.find("plate").findall("metadata")}
    assert plate["first_layer_print_sequence"] == "1 2"
    assert plate["first_layer_sequence_choice"] == "Customize"
    assert plate["plater_id"] == "1"


def test_thin_strokes_are_drawn_and_a_flat_part_gets_no_supports():
    _, settings, _ = decided(supported=False)
    assert settings["wall_generator"] == "arachne"
    assert settings["enable_support"] == "0"
    assert settings["different_settings_to_system"] == ["initial_layer_infill_speed;initial_layer_speed;wall_generator", "", "", ""]


def test_the_first_layer_is_printed_slowly():
    _, settings, _ = decided(supported=False)
    assert settings["initial_layer_speed"] == ["30", "30"]
    assert settings["initial_layer_infill_speed"] == ["60", "60"]


def test_a_supported_part_gets_normal_supports_from_the_plate_only():
    _, settings, _ = decided(supported=True)
    assert settings["enable_support"] == "1"
    assert settings["support_type"] == "normal(auto)"
    assert settings["support_on_build_plate_only"] == "1"
    assert settings["support_interface_top_layers"] == "3"
    assert settings["support_interface_spacing"] == "0.2"
    changed = settings["different_settings_to_system"][0].split(";")
    supports = {"enable_support", "support_type", "support_on_build_plate_only", "support_interface_top_layers", "support_interface_spacing"}
    assert set(changed) == supports | {"wall_generator", "initial_layer_speed", "initial_layer_infill_speed"}
    assert changed == sorted(changed)


def test_the_other_files_of_the_project_are_kept():
    files, _, _ = decided(supported=False)
    assert files["3D/3dmodel.model"] == b"<model/>"
    assert set(files) == set(exported_project())


def test_a_project_without_exactly_two_parts_is_refused():
    files = exported_project()
    files["Metadata/model_settings.config"] = MODEL_SETTINGS.replace('<part id="2"', '<part id="2"><mesh_stat/></part><part id="3"').encode()
    with pytest.raises(ValueError, match="2 parts"):
        bambu.decide(files, supported=False)


def test_first_layer_order_is_read_from_the_gcode():
    assert bambu.first_layer_order(GCODE) == [1, 2]
    assert bambu.first_layer_order(GCODE.replace("M620 S0A\nT1000", "M620 S1A\nT1000").replace("M620 S1A\nT1\n", "M620 S0A\nT0\n")) == [2, 1]


def test_command_without_the_flag_does_not_need_bambu_studio(tmp_path, monkeypatch):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    monkeypatch.setattr(bambu, "STUDIO", tmp_path / "missing")
    cli.main(["label-gauge"])
    assert not (tmp_path / "label-gauge-bambu.3mf").exists()


def test_the_flag_without_bambu_studio_says_so(tmp_path, monkeypatch):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    monkeypatch.setattr(bambu, "STUDIO", tmp_path / "missing")
    with pytest.raises(SystemExit, match="Bambu Studio"):
        cli.main(["label-gauge", "--bambu"])


@needs_bambu_studio
def test_bambu_studio_slices_the_gauge_project_cleanly(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    cli.main(["label-gauge", "--bambu"])

    project = zipfile.ZipFile(tmp_path / "label-gauge-bambu.3mf")
    settings = json.loads(project.read("Metadata/project_settings.config"))
    assert settings["printer_settings_id"] == "Bambu Lab P1S 0.4 nozzle"
    assert settings["print_settings_id"] == "0.20mm Standard @BBL X1C"
    assert settings["filament_settings_id"] == ["Overture Matte PLA @BBL X1C"] * 2
    assert settings["initial_layer_line_width"] == "0.5"
    assert settings["wall_generator"] == "arachne"
    assert settings["initial_layer_speed"] == ["30", "30"]
    assert not any(name.endswith(".gcode") for name in project.namelist())
    assert "Sliced by Bambu Studio" in capsys.readouterr().out


@needs_bambu_studio
def test_bambu_studio_supports_the_coupon_and_raises_no_warning(tmp_path, body, inlay):
    from drezimo_case.export import export_part

    neutral = next(path for path in export_part("coupon-a", body, inlay, tmp_path) if path.suffix == ".3mf")
    report = bambu.write_project(neutral, tmp_path / "coupon-a-bambu.3mf", supported=True)
    assert report.supports
    assert report.first_layer_order == [1, 2]
    assert report.minutes > 10 and len(report.grams) == 2

    with pytest.raises(bambu.SliceError, match="floating"):
        bambu.write_project(neutral, tmp_path / "unsupported.3mf", supported=False)
