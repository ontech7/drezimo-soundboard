# The macOS disk image's layout, for dmgbuild, which writes Finder's settings without driving Finder, so the layout
# holds when the image is built on CI. Usage, from app/:
#   dmgbuild -s src-tauri/macos/dmg.py -D app="<path to Drezimo Soundboard.app>" "Drezimo Soundboard" <output>.dmg
# The background comes from private/installer/render-dmg.mjs, with the same icon places.
import os.path

application = defines["app"]  # noqa: F821  # pyright: ignore[reportUndefinedVariable]
# dmgbuild runs this file without __file__: the paths are from app/
here = os.path.abspath("src-tauri/macos")

format = "UDZO"
files = [application]
symlinks = {"Applications": "/Applications"}
icon = os.path.join(here, "..", "icons", "icon.icns")

# dmgbuild adds dmg-background@2x.png beside it for Retina screens
background = os.path.join(here, "dmg-background.png")
window_rect = ((200, 120), (660, 400))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False

icon_size = 112
text_size = 13
icon_locations = {os.path.basename(application): (170, 190), "Applications": (490, 190)}
