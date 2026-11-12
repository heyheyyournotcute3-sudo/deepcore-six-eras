#!/usr/bin/env python3
"""Stage 3: write the RON files from the generated tables."""
import os, json

HERE = os.path.dirname(__file__)
OUT = os.path.join(HERE, "..", "..", "assets", "data")
os.makedirs(OUT, exist_ok=True)
s1 = json.load(open("/tmp/content_stage1.json"))
s2 = json.load(open("/tmp/content_stage2.json"))


def f(x):
    return f"{x:.4f}".rstrip("0").rstrip(".") if isinstance(x, float) else str(x)


def rgb(c):
    return "(" + ",".join(f(v) for v in c) + ")"


# --- blocks.ron -------------------------------------------------------------
lines = ["// Block definitions. Generated from tools/content_gen/gen_content.py.", "["]
for b in s1["blocks"]:
    drops = "[" + ",".join(f'("{i}",{n})' for i, n in b["drops"]) + "]"
    parts = [
        f'id: "{b["id"]}"',
        f'hardness: {f(b["hardness"])}',
        f'min_tier: {b["min_tier"]}',
        f"drops: {drops}",
        f"color: {rgb(b['color'])}",
    ]
    pat = b["pattern"]
    if pat.startswith("OreFleck"):
        parts.append("pattern: " + pat)
    else:
        parts.append(f"pattern: {pat}")
    if b["emissive"]:
        parts.append(f'emissive: {f(b["emissive"])}')
    if b["radiation"]:
        r = b["radiation"]
        parts.append(f"radiation: Some((radius: {f(r[0])}, dose_per_sec: {f(r[1])}))")
    if b["layer"] is not None:
        parts.append(f'layer_hint: Some({b["layer"]})')
    for flag, key in [("liquid", "liquid"), ("unbreakable", "unbreakable"), ("passable", "passable")]:
        if b[flag]:
            parts.append(f"{key}: true")
    if b["machine"]:
        parts.append(f'machine: Some("{b["machine"]}")')
    lines.append("    (" + ", ".join(parts) + "),")
lines.append("]")
open(os.path.join(OUT, "blocks.ron"), "w").write("\n".join(lines) + "\n")

# --- items.ron --------------------------------------------------------------
lines = ["// Item definitions. Generated from tools/content_gen/gen_content.py.", "["]
for it in s1["items"]:
    cat = it["cat"]
    if cat == "Tool":
        tier, speed, area, energy, reach = it["tool"]
        ekind, ecap = energy
        if ekind == "None":
            e = "None"
        elif ekind == "CoreCell":
            e = f'CoreCell(breaks: {ecap})'
        else:
            e = f'{ekind}(capacity: {ecap})'
        cat = (f'Tool((tier: {tier}, speed: {speed}, area: ({area[0]},{area[1]}), '
               f'energy: {e}, reach_bonus: {f(reach)}))')
    parts = [
        f'id: "{it["id"]}"',
        f'name: "{it["name"]}"',
        f'stack: {it["stack"]}',
        f"category: {cat}",
    ]
    if it["desc"]:
        parts.append(f'description: "{it["desc"]}"')
    lines.append("    (" + ", ".join(parts) + "),")
lines.append("]")
open(os.path.join(OUT, "items.ron"), "w").write("\n".join(lines) + "\n")

# --- machines.ron -----------------------------------------------------------
lines = ["// Machine definitions. Generated from tools/content_gen/gen_content.py.", "["]
for m in s2["machines"]:
    parts = [
        f'id: "{m["id"]}"',
        f'name: "{m["name"]}"',
        f'size: ({m["size"][0]},{m["size"][1]},{m["size"][2]})',
        f"energy: {m['energy']}",
        f'input_slots: {m["ins"]}',
        f'output_slots: {m["outs"]}',
        "build_cost: [" + ",".join(f'("{i}",{n})' for i, n in m["cost"]) + "]",
    ]
    if m["tech"]:
        parts.append(f'tech: Some("{m["tech"]}")')
    if m["cascade"]:
        parts.append(f"cascade: Some((min: {m['cascade'][0]}, max: {m['cascade'][1]}))")
    if m["upgrades"]:
        parts.append(f'upgrade_slots: {m["upgrades"]}')
    if m["producer"]:
        parts.append("producer: true")
    if m["reactor"]:
        r = m["reactor"]
        parts.append(
            "reactor: Some((output_kw: {}, warn_temp: {}, limit_temp: {}, "
            "heat_per_rod: {}, cooling_rate: {}))".format(*[f(v) for v in r])
        )
    lines.append("    (" + ", ".join(parts) + "),")
lines.append("]")
open(os.path.join(OUT, "machines.ron"), "w").write("\n".join(lines) + "\n")

# --- recipes.ron ------------------------------------------------------------
lines = ["// Recipe definitions. Generated from tools/content_gen/gen_content.py.", "["]
for r in s2["recipes"]:
    parts = [
        f'id: "{r["id"]}"',
        f'machine: "{r["machine"]}"',
        f'time: {f(r["time"])}',
        "inputs: [" + ",".join(f'("{i}",{n})' for i, n in r["ins"]) + "]",
        "outputs: [" + ",".join(f'("{i}",{n})' for i, n in r["outs"]) + "]",
    ]
    if r["tech"]:
        parts.append(f'tech: Some("{r["tech"]}")')
    if r["hand"]:
        parts.append("hand: true")
    lines.append("    (" + ", ".join(parts) + "),")
lines.append("]")
open(os.path.join(OUT, "recipes.ron"), "w").write("\n".join(lines) + "\n")

# --- tech_tree.ron ----------------------------------------------------------
lines = ["// Tech tree. Generated from tools/content_gen/gen_content.py.", "["]
for t in s2["tech"]:
    parts = [
        f'id: "{t["id"]}"',
        f'name: "{t["name"]}"',
        f'era: {t["era"]}',
        "parents: [" + ",".join(f'"{p}"' for p in t["parents"]) + "]",
        "cost: [" + ",".join(f'("{i}",{n})' for i, n in t["cost"]) + "]",
        "unlocks: [" + ",".join(f'"{u}"' for u in t["unlocks"]) + "]",
        f'description: "{t["desc"]}"',
    ]
    lines.append("    (" + ", ".join(parts) + "),")
lines.append("]")
open(os.path.join(OUT, "tech_tree.ron"), "w").write("\n".join(lines) + "\n")

# --- layers.ron -------------------------------------------------------------
lines = ["// World layers. Generated from tools/content_gen/gen_content.py.", "["]
for l in s2["layers"]:
    parts = [
        f'id: "{l["id"]}"',
        f'name: "{l["name"]}"',
        f'depth: ({l["depth"][0]},{l["depth"][1]})',
        f'base_block: "{l["base_block"]}"',
        "mix: [" + ",".join(f'("{b}",{f(w)})' for b, w in l["mix"]) + "]",
        "ores: [" + ",".join(
            '(block: "{}", frequency: {}, vein_size: ({},{}))'.format(
                o["block"], f(o["frequency"]), o["vein_size"][0], o["vein_size"][1])
            for o in l["ores"]) + "]",
    ]
    haz = []
    for h in l["hazards"]:
        if h["kind"] == "flooding":
            haz.append(f'Flooding(pocket_chance: {f(h["pocket_chance"])}, max_volume: {h["max_volume"]})')
        elif h["kind"] == "gas":
            haz.append(f'Gas(pocket_chance: {f(h["pocket_chance"])}, oxygen_per_sec: {f(h["oxygen_per_sec"])})')
        elif h["kind"] == "heat":
            haz.append(f'Heat(per_sec: {f(h["per_sec"])})')
        elif h["kind"] == "cave_in":
            haz.append(f'CaveIn(max_span: {h["max_span"]})')
        elif h["kind"] == "radiation":
            haz.append(f'Radiation(per_sec: {f(h["per_sec"])})')
    parts.append("hazards: [" + ",".join(haz) + "]")
    parts.append(f'ambient_temp: {f(l["ambient_temp"])}')
    parts.append(f'cave_density: {f(l["cave_density"])}')
    lines.append("    (" + ", ".join(parts) + "),")
lines.append("]")
open(os.path.join(OUT, "layers.ron"), "w").write("\n".join(lines) + "\n")

print("wrote RON files to", os.path.abspath(OUT))
for fn in sorted(os.listdir(OUT)):
    print(" ", fn, os.path.getsize(os.path.join(OUT, fn)), "bytes")
