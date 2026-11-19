import json
import zipfile
from xml.etree import ElementTree

import pytest
from helpers import nominal

from drezimo_case import __main__ as cli
from drezimo_case import bambu
from drezimo_case.board import Rect

needs_bambu_studio = pytest.mark.skipif(not bambu.STUDIO.exists(), reason="Bambu Studio is not installed")

MODEL_SETTINGS = """<?xml version="1.0" encoding="UTF-8"?>
<config>
  <object id="3">
    <metadata key="name" value="inlay-gauge"/>
    <metadata key="extruder" value="1"/>
    <part id="1" subtype="normal_part">
      <metadata key="name" value="inlay-gauge"/>
      <mesh_stat face_count="12"/>
    </part>
    <part id="2" subtype="normal_part">
      <metadata key="name" value="inlay-gauge_2"/>
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


def decided(supported, solid_deck=False, walls_show=False):
    files = bambu.decide(exported_project(), supported, solid_deck, walls_show)
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
    black = {"support_filament", "support_interface_filament"}
    assert set(changed) == supports | black | {"wall_generator", "initial_layer_speed", "initial_layer_infill_speed"}
    assert changed == sorted(changed)


def test_supports_are_printed_in_the_bodys_black():
    # Left to Bambu Studio they start in white, a third of a millimetre from the rim of the well on the face that shows.
    _, settings, _ = decided(supported=True)
    assert (settings["support_filament"], settings["support_interface_filament"]) == ("1", "1")


def test_a_deck_that_must_not_give_is_printed_full_through():
    # Bambu Studio's own 0.5 and 0.8 mm skins leave 0.7 mm of a 2 mm deck sparse.
    skins = ("bottom_shell_layers", "bottom_shell_thickness", "top_shell_layers", "top_shell_thickness")
    _, settings, _ = decided(supported=True, solid_deck=True)
    assert [settings[key] for key in skins] == ["10", "1", "10", "1"]
    changed = settings["different_settings_to_system"][0].split(";")
    assert set(skins) <= set(changed) and changed == sorted(changed)

    _, settings, _ = decided(supported=True)
    assert not set(skins) & set(settings)


def test_the_shell_and_the_bottom_are_pressed_with_a_finger():
    assert {name: part.solid_deck for name, part in cli.PARTS.items()} == {"coupon-a": False, "inlay-gauge": False, "shell": True, "bottom": True}


def test_a_part_that_is_full_plastic_has_no_sparse_middle_anywhere():
    # Under the USB module the bottom's plate is 3.4 mm thick: two skins of 1 mm left 1.4 mm of it sparse.
    settings, _ = one_colour(solid_deck=True, full=True)
    assert settings["bottom_shell_layers"] == "10"
    settings, _ = one_colour(full=True)
    # Bambu Studio refuses its own grid at 100 %: lines side by side, as it fills any full layer.
    assert (settings["sparse_infill_density"], settings["sparse_infill_pattern"]) == ("100%", "zig-zag")
    assert settings["different_settings_to_system"][0] == "sparse_infill_density;sparse_infill_pattern"
    assert "sparse_infill_density" not in one_colour()[0]
    assert {name: part.full for name, part in cli.PARTS.items()} == {"coupon-a": False, "inlay-gauge": False, "shell": False, "bottom": True}




def test_walls_that_show_get_their_seam_at_the_back_and_no_brim():
    _, settings, _ = decided(supported=True, walls_show=True)
    assert (settings["seam_position"], settings["brim_type"]) == ("back", "no_brim")
    changed = settings["different_settings_to_system"][0].split(";")
    assert {"seam_position", "brim_type"} <= set(changed) and changed == sorted(changed)

    _, settings, _ = decided(supported=True)
    assert "seam_position" not in settings and "brim_type" not in settings
    # The bottom's edge has to enter the shell's rim: no brim welded to it either.
    assert {name: part.walls_show for name, part in cli.PARTS.items()} == {"coupon-a": False, "inlay-gauge": False, "shell": True, "bottom": True}


def sliced(tmp_path, grams):
    """A folder as Bambu Studio leaves it after slicing."""
    plate = {"warning_message": "", "total_predication": 600, "filaments": [{"total_used_g": g} for g in grams]}
    (tmp_path / "result.json").write_text(json.dumps({"sliced_plates": [plate]}))
    (tmp_path / "plate_1.gcode").write_text(GCODE)
    return tmp_path, {"3D/3dmodel.model": b"<model>BambuStudio-02.09.01.70</model>"}


def test_a_slice_with_more_white_than_black_is_refused(tmp_path):
    # The body and the inlay loaded the wrong way round: a white case with black letters.
    assert bambu._report(*sliced(tmp_path, (19.9, 6.1)), None).grams == (19.9, 6.1)
    with pytest.raises(bambu.SliceError, match="white"):
        bambu._report(*sliced(tmp_path, (6.1, 19.9)), None)



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


def test_the_filaments_of_the_supports_are_read_from_the_gcode():
    assert bambu.support_filaments(GCODE) == set()
    white_wall = "; FEATURE: Outer wall\nG1 X30 Y10 E.5"
    in_white = GCODE.replace(white_wall, f"; FEATURE: Support\nG1 X29 Y10 E.5\n; FEATURE: Support interface\nG1 X29 Y11 E.5\n{white_wall}")
    assert bambu.support_filaments(in_white) == {2}
    assert bambu.support_filaments(in_white.replace("; FEATURE: Outer wall\nG1 X20", "; FEATURE: Support\nG1 X20")) == {1, 2}


def with_a_support(at):
    white_wall = "; FEATURE: Outer wall\nG1 X30 Y10 E.5"
    return GCODE.replace(white_wall, f"; FEATURE: Support\nG1 {at} E.5\n{white_wall}")


def test_where_the_supports_stand_is_read_from_the_gcode():
    # About the middle of the part's first layer, which in the sample runs from x 20 to x 30 at y 10.
    assert bambu.support_points(GCODE) == []
    assert bambu.support_points(with_a_support("X29 Y11")) == [pytest.approx((4.0, 1.0))]
    assert bambu.support_points(with_a_support("Y14")) == [pytest.approx((-15.0, 4.0))]


def test_filament_purged_away_from_the_part_is_not_part_of_it():
    # At a colour change the head goes to the chute and pushes filament out without moving, under the last feature's name.
    purged = with_a_support("X29 Y11").replace("M620 S1A\nT1\n", "M620 S1A\nT1\nG1 X250 Y250 F9000\nG1 E20 F300\nG1 E.8\n")
    assert bambu.support_points(purged) == [pytest.approx((4.0, 1.0))]
    assert bambu.first_layer_order(purged) == [1, 2]


def test_supports_outside_the_areas_a_part_allows_are_refused():
    supported = with_a_support("X29 Y11")
    bambu.check_supports(supported, [Rect(-9, -9, -8, -8), Rect(3, 0, 5, 2)])
    bambu.check_supports(supported, [Rect(0, 0, 3.6, 2)])
    bambu.check_supports(supported, None)
    with pytest.raises(bambu.SliceError, match="supports.*1.0 mm"):
        bambu.check_supports(supported, [Rect(-5, -5, 3.0, 5)])


def test_each_part_is_printed_with_a_nozzle_that_has_its_presets():
    # Only the inlays need the fine nozzle, but the owner prints every part with it: changing nozzle is no quick job.
    assert {name: part.nozzle for name, part in cli.PARTS.items()} == {"coupon-a": 0.2, "inlay-gauge": 0.2, "shell": 0.2, "bottom": 0.2}
    assert set(bambu.NOZZLES) == {0.4, 0.2}


@needs_bambu_studio
def test_the_presets_of_each_nozzle_go_with_its_printer():
    for size, nozzle in bambu.NOZZLES.items():
        assert bambu.resolve_preset(nozzle.printer)["nozzle_diameter"] == [str(size)]
        for preset in (nozzle.process, nozzle.filament):
            assert nozzle.printer in bambu.resolve_preset(preset)["compatible_printers"]


def test_command_without_the_flag_does_not_need_bambu_studio(tmp_path, monkeypatch):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    monkeypatch.setattr(bambu, "STUDIO", tmp_path / "missing")
    cli.main(["coupon-a"])
    assert not (tmp_path / "coupon-a-bambu.3mf").exists()


def test_the_flag_without_bambu_studio_says_so(tmp_path, monkeypatch):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    monkeypatch.setattr(bambu, "STUDIO", tmp_path / "missing")
    with pytest.raises(SystemExit, match="Bambu Studio"):
        cli.main(["inlay-gauge", "--bambu"])


@needs_bambu_studio
def test_bambu_studio_slices_the_gauge_project_cleanly_for_the_0_2_mm_nozzle(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(cli, "PRINT_DIR", tmp_path)
    monkeypatch.setattr(cli, "MEASUREMENTS", nominal())
    cli.main(["inlay-gauge", "--bambu"])

    project = zipfile.ZipFile(tmp_path / "inlay-gauge-bambu.3mf")
    settings = json.loads(project.read("Metadata/project_settings.config"))
    assert settings["printer_settings_id"] == "Bambu Lab P1S 0.2 nozzle"
    assert settings["print_settings_id"] == "0.10mm Standard @BBL X1C 0.2 nozzle"
    assert settings["filament_settings_id"] == ["Overture Matte PLA @BBL X1C 0.2 nozzle"] * 2
    assert settings["nozzle_diameter"] == ["0.2"]
    assert (settings["layer_height"], settings["initial_layer_line_width"]) == ("0.1", "0.25")
    assert settings["wall_generator"] == "arachne"
    assert settings["initial_layer_speed"] == ["30", "30"]
    assert not any(name.endswith(".gcode") for name in project.namelist())
    said = capsys.readouterr().out
    assert "Sliced by Bambu Studio" in said and "for the 0.2 mm nozzle" in said


@needs_bambu_studio
def test_bambu_studio_supports_the_coupon_and_raises_no_warning(tmp_path, body, inlay):
    from drezimo_case.export import export_part

    neutral = next(path for path in export_part("coupon-a", body, inlay, tmp_path) if path.suffix == ".3mf")
    report = bambu.write_project(neutral, tmp_path / "coupon-a-bambu.3mf", supported=True, nozzle=0.2)
    assert report.supports
    assert report.first_layer_order == [1, 2]
    assert report.minutes > 10 and len(report.grams) == 2

    settings = json.loads(zipfile.ZipFile(tmp_path / "coupon-a-bambu.3mf").read("Metadata/project_settings.config"))
    assert settings["printer_settings_id"] == "Bambu Lab P1S 0.2 nozzle"
    assert settings["print_settings_id"] == "0.10mm Standard @BBL X1C 0.2 nozzle"
    assert settings["filament_settings_id"] == ["Overture Matte PLA @BBL X1C 0.2 nozzle"] * 2
    assert (settings["support_top_z_distance"], settings["support_line_width"]) == ("0.1", "0.22")
    assert (settings["support_filament"], settings["support_interface_filament"]) == ("1", "1")

    with pytest.raises(bambu.SliceError, match="floating"):
        bambu.write_project(neutral, tmp_path / "unsupported.3mf", supported=False, nozzle=0.2)


@needs_bambu_studio
def test_bambu_studio_supports_the_shell_only_under_its_wells(tmp_path, shell, p):
    from drezimo_case.export import export_part
    from drezimo_case.shell import support_areas

    neutral = next(path for path in export_part("shell", *shell, tmp_path, turned=True) if path.suffix == ".3mf")
    report = bambu.write_project(
        neutral, tmp_path / "shell-bambu.3mf", supported=True, nozzle=0.2, supports_within=support_areas(p), solid_deck=True, walls_show=True
    )
    assert report.supports
    assert report.first_layer_order == [1, 2]
    assert 800 < report.minutes < 1000
    # 78 g with Bambu Studio's own skins.
    assert report.grams[0] > 82

    assert report.height == pytest.approx(p.deck_top - p.wall_bottom)

    with pytest.raises(bambu.SliceError, match="supports"):
        bambu.write_project(neutral, tmp_path / "wrong.3mf", supported=True, nozzle=0.2, supports_within=support_areas(p)[:1])


ONE_PART = MODEL_SETTINGS.replace(
    """    <part id="2" subtype="normal_part">
      <metadata key="name" value="inlay-gauge_2"/>
      <mesh_stat face_count="12"/>
    </part>
""",
    "",
)
BLACK_ONLY = """M620 S0A
T1000
; CHANGE_LAYER
; FEATURE: Outer wall
G1 X20 Y10 E.5
; CHANGE_LAYER
G1 X40 Y10 E.5
M620 S255A
"""


def one_colour(**options):
    files = exported_project() | {"Metadata/model_settings.config": ONE_PART.encode()}
    files = bambu.decide(files, supported=False, inlaid=False, **options)
    return json.loads(files["Metadata/project_settings.config"]), ElementTree.fromstring(files["Metadata/model_settings.config"])


def test_a_part_of_one_colour_is_its_body_alone_in_black():
    settings, model = one_colour()
    assert settings["filament_colour"] == ["#1A1A1C"]
    assert settings["flush_volumes_matrix"] == ["0"]
    assert settings["curr_bed_type"] == "Textured PEI Plate"
    parts = [{m.get("key"): m.get("value") for m in part.findall("metadata")} for part in model.iter("part")]
    assert parts == [{"name": "body", "extruder": "1"}]
    assert "first_layer_print_sequence" not in {m.get("key") for m in model.find("plate").findall("metadata")}


def test_what_the_inlays_ask_of_the_printer_is_not_asked_without_them():
    settings, _ = one_colour()
    assert settings["wall_generator"] == "classic"
    assert "initial_layer_speed" not in settings
    assert settings["different_settings_to_system"] == ["", "", "", ""]


def test_a_part_whose_heights_need_them_is_printed_in_its_own_layers():
    settings, _ = one_colour(layer_height=0.1)
    assert settings["layer_height"] == "0.1"
    assert settings["different_settings_to_system"][0] == "layer_height"
    assert settings == one_colour()[0] | {"layer_height": "0.1", "different_settings_to_system": ["layer_height", "", "", ""]}
    assert {name: part.layers for name, part in cli.PARTS.items()} == {"coupon-a": None, "inlay-gauge": None, "shell": None, "bottom": 0.1}


def test_how_high_the_layers_end_is_read_from_the_gcode():
    assert bambu.printed_height("; CHANGE_LAYER\n; Z_HEIGHT: 0.2\nG1 X1 E.5\n; CHANGE_LAYER\n; Z_HEIGHT: 0.3\nG1 X2 E.5\n") == 0.3


def test_a_slice_that_ends_short_of_the_part_is_refused(tmp_path):
    # Layers that do not divide a part's height leave its top a part of a layer short.
    folder, imported = sliced(tmp_path, (61.5,))
    (folder / "plate_1.gcode").write_text(BLACK_ONLY.replace("; CHANGE_LAYER", "; CHANGE_LAYER\n; Z_HEIGHT: 10.8"))
    assert bambu._report(folder, imported, None, inlaid=False, height=10.8).height == 10.8
    with pytest.raises(bambu.SliceError, match="end at 10.8 mm on a part 10.9 mm high"):
        bambu._report(folder, imported, None, inlaid=False, height=10.9)


def test_a_part_of_one_colour_with_two_parts_is_refused():
    with pytest.raises(ValueError, match="1 part"):
        bambu.decide(exported_project(), supported=False, inlaid=False)


def test_the_slice_of_a_part_of_one_colour_is_read(tmp_path):
    folder, imported = sliced(tmp_path, (61.5,))
    (folder / "plate_1.gcode").write_text(BLACK_ONLY)
    report = bambu._report(folder, imported, None, inlaid=False)
    assert report.grams == (61.5,)
    assert report.first_layer_order == [1]
    assert not report.supports


@needs_bambu_studio
def test_bambu_studio_slices_the_bottom_with_no_supports_and_no_warning(tmp_path, bottom, p):
    from drezimo_case.export import export_part

    neutral = next(path for path in export_part("bottom", bottom, None, tmp_path, upright=True) if path.suffix == ".3mf")
    report = bambu.write_project(
        neutral, tmp_path / "bottom-bambu.3mf", supported=False, nozzle=0.2, solid_deck=True, walls_show=True, inlaid=False, layer_height=0.1, full=True
    )
    assert report.height == pytest.approx(p.standoff_top - p.wall_bottom)
    assert not report.supports
    assert report.first_layer_order == [1]
    assert len(report.grams) == 1 and 50 < report.grams[0] < 120

    settings = json.loads(zipfile.ZipFile(tmp_path / "bottom-bambu.3mf").read("Metadata/project_settings.config"))
    assert settings["printer_settings_id"] == "Bambu Lab P1S 0.2 nozzle"
    assert settings["print_settings_id"] == "0.10mm Standard @BBL X1C 0.2 nozzle"
    assert settings["filament_settings_id"] == ["Overture Matte PLA @BBL X1C 0.2 nozzle"]
    assert (settings["layer_height"], settings["enable_support"], settings["brim_type"]) == ("0.1", "0", "no_brim")
    # Ten layers from each face, and what lies between them full too.
    assert (settings["bottom_shell_layers"], settings["top_shell_layers"]) == ("10", "10")
    assert (settings["sparse_infill_density"], settings["sparse_infill_pattern"]) == ("100%", "zig-zag")
