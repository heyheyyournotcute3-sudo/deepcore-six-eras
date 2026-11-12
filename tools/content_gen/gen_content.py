#!/usr/bin/env python3
"""One-off authoring aid that emits the RON content files.

The RON files under assets/data/ are the source of truth at runtime; this
script exists so the item/recipe/tech cross references were generated from one
consistent table instead of by hand. Re-run it only when adding whole content
groups: `python3 tools/content_gen/gen_content.py`.
"""
import os, json, textwrap

OUT = os.path.join(os.path.dirname(__file__), "..", "..", "assets", "data")

# ---------------------------------------------------------------------------
# Blocks
# ---------------------------------------------------------------------------
# (id, hardness, min_tier, drops, color, pattern, emissive, radiation, layer_hint, flags)
B = []
def block(id, hardness, tier, drops, color, pattern="Speckle", emissive=0.0,
          radiation=None, layer=None, liquid=False, unbreakable=False,
          passable=False, machine=None):
    B.append(dict(id=id, hardness=hardness, min_tier=tier, drops=drops, color=color,
                  pattern=pattern, emissive=emissive, radiation=radiation, layer=layer,
                  liquid=liquid, unbreakable=unbreakable, passable=passable, machine=machine))

block("air", 0.0, 1, [], (0.0, 0.0, 0.0), "Solid", passable=True)
block("bedrock", 1.0, 9, [], (0.10, 0.10, 0.12), "Speckle", unbreakable=True)
block("grass", 0.6, 1, [("dirt", 1)], (0.30, 0.55, 0.22), "Speckle", layer=0)
block("dirt", 0.6, 1, [("dirt", 1)], (0.42, 0.30, 0.19), "Speckle", layer=0)
block("gravel", 0.7, 1, [("gravel", 1)], (0.50, 0.48, 0.46), "Speckle", layer=0)
block("clay", 0.8, 1, [("clay_lump", 1)], (0.60, 0.45, 0.36), "Speckle", layer=0)
block("sand", 0.6, 1, [("sand", 1)], (0.80, 0.74, 0.50), "Speckle", layer=0)
block("stone", 1.6, 1, [("stone_cobble", 1)], (0.52, 0.52, 0.54), "Speckle", layer=1)
block("limestone", 1.4, 1, [("limestone", 1)], (0.70, 0.68, 0.60), "Bands", layer=1)
block("shale", 1.5, 1, [("stone_cobble", 1)], (0.36, 0.36, 0.40), "Bands", layer=1)
block("granite", 2.5, 2, [("granite_cobble", 1)], (0.55, 0.50, 0.48), "Speckle", layer=2)
block("basalt", 3.4, 3, [("basalt_cobble", 1)], (0.22, 0.22, 0.26), "Speckle", layer=3)
block("gneiss", 4.6, 4, [("gneiss_cobble", 1)], (0.40, 0.38, 0.44), "Bands", layer=4)
block("obsidian", 5.2, 4, [("obsidian_shard", 1)], (0.12, 0.10, 0.18), "Solid", layer=4)
block("mantle_rock", 6.5, 5, [("mantle_fragment", 1)], (0.34, 0.14, 0.12), "Speckle", layer=5)

# ores
block("coal_seam", 1.8, 1, [("coal", 2)], (0.30, 0.30, 0.32), "OreFleck((0.08,0.08,0.08))", layer=1)
block("malachite", 2.0, 1, [("raw_copper", 1)], (0.52, 0.52, 0.54), "OreFleck((0.20,0.75,0.55))", layer=1)
block("cassiterite", 2.2, 1, [("raw_tin", 1)], (0.52, 0.52, 0.54), "OreFleck((0.75,0.75,0.80))", layer=1)
block("hematite", 2.6, 1, [("raw_iron", 1)], (0.52, 0.52, 0.54), "OreFleck((0.65,0.30,0.22))", layer=1)
block("galena", 2.8, 2, [("raw_lead", 1)], (0.55, 0.50, 0.48), "OreFleck((0.45,0.45,0.55))", layer=2)
block("sphalerite", 2.8, 2, [("raw_zinc", 1)], (0.55, 0.50, 0.48), "OreFleck((0.70,0.65,0.40))", layer=2)
block("sulfur_deposit", 2.4, 2, [("sulfur", 2)], (0.55, 0.50, 0.48), "OreFleck((0.90,0.85,0.25))", layer=2)
block("quartz_vein", 2.6, 2, [("quartz", 1)], (0.55, 0.50, 0.48), "OreFleck((0.92,0.92,0.98))", layer=2)
block("bauxite", 2.9, 2, [("raw_bauxite", 1)], (0.55, 0.50, 0.48), "OreFleck((0.85,0.55,0.35))", layer=2)
block("rock_salt", 2.2, 2, [("salt", 2)], (0.55, 0.50, 0.48), "OreFleck((0.95,0.95,0.92))", layer=2)
block("pentlandite", 3.6, 3, [("raw_nickel", 1)], (0.22, 0.22, 0.26), "OreFleck((0.60,0.70,0.55))", layer=3)
block("wolframite", 4.0, 3, [("raw_tungsten", 1)], (0.22, 0.22, 0.26), "OreFleck((0.35,0.30,0.28))", layer=3)
block("chromite", 3.8, 3, [("raw_chromium", 1)], (0.22, 0.22, 0.26), "OreFleck((0.45,0.35,0.55))", layer=3)
block("graphite_vein", 3.2, 3, [("graphite", 2)], (0.22, 0.22, 0.26), "OreFleck((0.10,0.10,0.12))", layer=3)
block("fluorite", 3.4, 3, [("fluorite", 1)], (0.22, 0.22, 0.26), "Glow", emissive=0.4, layer=3)
block("zircon", 4.2, 3, [("raw_zircon", 1)], (0.22, 0.22, 0.26), "OreFleck((0.80,0.70,0.50))", layer=3)
block("gold_vein", 4.0, 3, [("raw_gold", 1)], (0.22, 0.22, 0.26), "OreFleck((0.95,0.78,0.20))", layer=3)
block("pitchblende", 6.0, 4, [("uranium_ore", 1)], (0.15, 0.18, 0.12), "OreFleck((0.40,0.90,0.30))",
      emissive=0.3, radiation=(4.0, 0.5), layer=4)
block("monazite", 5.0, 4, [("monazite_sand", 1)], (0.40, 0.38, 0.44), "OreFleck((0.85,0.55,0.30))", layer=4)
block("dense_pitchblende", 7.0, 5, [("uranium_ore", 3)], (0.12, 0.16, 0.10), "OreFleck((0.45,1.00,0.35))",
      emissive=0.6, radiation=(6.0, 1.2), layer=5)
block("core_crystal", 8.0, 5, [("core_crystal", 1)], (0.30, 0.60, 0.90), "Glow", emissive=1.0,
      radiation=(3.0, 0.3), layer=5)

# fluids / gas
block("water", 0.0, 1, [], (0.15, 0.35, 0.70), "Solid", liquid=True, passable=True)
block("methane", 0.0, 1, [], (0.55, 0.60, 0.35), "Solid", passable=True)

# ---------------------------------------------------------------------------
# Player-built blocks (no machine behaviour)
# ---------------------------------------------------------------------------
block("timber_block", 1.2, 1, [("timber", 1)], (0.55, 0.40, 0.24), "Bands")
block("timber_support", 1.2, 1, [("timber", 1)], (0.50, 0.36, 0.20), "Bands")
block("steel_support", 3.0, 3, [("steel_plate", 1)], (0.60, 0.62, 0.66), "Solid")
block("ladder", 0.4, 1, [("ladder", 1)], (0.60, 0.45, 0.25), "Solid", passable=True)
block("brick_block", 2.0, 1, [("brick", 1)], (0.62, 0.32, 0.25), "Bands")
block("fire_brick_block", 2.8, 2, [("fire_brick", 1)], (0.70, 0.40, 0.30), "Bands")
block("glass_block", 0.5, 1, [("glass", 1)], (0.80, 0.90, 0.95), "Solid", emissive=0.05)
block("steel_block", 4.0, 3, [("steel_plate", 2)], (0.55, 0.57, 0.62), "Solid")
block("lead_block", 3.2, 3, [("lead_plate", 2)], (0.45, 0.45, 0.50), "Solid")
block("dense_shielding", 4.5, 4, [("dense_shielding_plate", 1)], (0.30, 0.30, 0.36), "Solid")
block("graphite_moderator", 3.0, 4, [("graphite_block", 1)], (0.14, 0.14, 0.16), "Speckle")
block("reactor_casing", 5.0, 4, [("stainless_casing", 1)], (0.68, 0.70, 0.74), "Solid")
block("oil_lamp", 0.5, 1, [("oil_lamp", 1)], (0.85, 0.65, 0.25), "Glow", emissive=0.8)
block("electric_lamp", 0.5, 1, [("electric_lamp", 1)], (0.90, 0.92, 1.00), "Glow", emissive=1.0)
block("cable_block", 0.4, 1, [("copper_wire", 1)], (0.70, 0.45, 0.25), "Solid", passable=True)
block("pipe_block", 0.6, 2, [("copper_pipe", 1)], (0.75, 0.50, 0.30), "Solid", passable=True)
block("rail_block", 0.8, 2, [("rail", 1)], (0.50, 0.50, 0.55), "Solid")
block("conveyor_block", 0.8, 2, [("conveyor", 1)], (0.35, 0.35, 0.38), "Solid")
block("scaffold", 1.0, 1, [("timber", 2)], (0.55, 0.45, 0.30), "Bands", passable=True)

# machine blocks
MACHINE_BLOCKS = [
    ("workbench", "workbench"), ("storage_crate", "storage_crate"),
    ("research_desk", "research_desk"), ("brick_kiln", "brick_kiln"),
    ("stone_furnace", "stone_furnace"), ("anvil", "anvil"),
    ("blast_furnace_block", "blast_furnace"), ("hand_pump", "hand_pump"),
    ("steam_boiler", "steam_boiler"), ("steam_engine", "steam_engine"),
    ("crusher", "crusher"), ("ore_washer", "ore_washer"),
    ("mine_lift", "mine_lift"), ("steam_turbine", "steam_turbine"),
    ("generator", "generator"), ("battery_bank", "battery_bank"),
    ("arc_furnace", "arc_furnace"), ("electrolysis_cell", "electrolysis_cell"),
    ("circuit_assembler", "circuit_assembler"), ("charging_station", "charging_station"),
    ("vent_fan", "vent_fan"), ("electric_pump", "electric_pump"),
    ("chemical_plant", "chemical_plant"), ("alloy_furnace", "alloy_furnace"),
    ("machine_shop", "machine_shop"), ("rare_earth_separator", "rare_earth_separator"),
    ("decon_shower", "decon_shower"), ("leaching_tank", "leaching_tank"),
    ("precipitator", "precipitator"), ("conversion_plant", "conversion_plant"),
    ("centrifuge", "centrifuge"), ("fuel_fabricator", "fuel_fabricator"),
    ("reactor_block", "reactor"), ("reactor_console", "reactor_console"),
    ("scram_panel", "scram_panel"), ("cooling_pool", "cooling_pool"),
    ("waste_vault", "waste_vault"), ("coolant_pump", "coolant_pump"),
    ("heat_exchanger", "heat_exchanger"),
]
for bid, machine in MACHINE_BLOCKS:
    block(bid, 2.5, 1, [(bid, 1)], (0.55, 0.55, 0.58), "Solid", machine=machine)

# ---------------------------------------------------------------------------
# Items
# ---------------------------------------------------------------------------
I = []
def item(id, name, stack=64, cat="Material", desc="", tool=None):
    I.append(dict(id=id, name=name, stack=stack, cat=cat, desc=desc, tool=tool))

# raws
item("dirt", "Dirt", cat="Raw")
item("gravel", "Gravel", cat="Raw", desc="Crushed stone; used for concrete and filtering.")
item("sand", "Sand", cat="Raw")
item("clay_lump", "Clay", cat="Raw")
item("timber", "Timber", cat="Raw", desc="Chopped from the trees around the camp.")
item("stone_cobble", "Stone", cat="Raw")
item("limestone", "Limestone", cat="Raw", desc="Flux for the blast furnace.")
item("granite_cobble", "Granite", cat="Raw")
item("basalt_cobble", "Basalt", cat="Raw")
item("gneiss_cobble", "Gneiss", cat="Raw")
item("obsidian_shard", "Obsidian Shard", cat="Raw")
item("mantle_fragment", "Mantle Fragment", cat="Raw")
item("coal", "Coal", cat="Raw")
item("raw_copper", "Raw Copper", cat="Raw")
item("raw_tin", "Raw Tin", cat="Raw")
item("raw_iron", "Raw Iron", cat="Raw")
item("raw_lead", "Raw Lead", cat="Raw")
item("raw_zinc", "Raw Zinc", cat="Raw")
item("sulfur", "Sulfur", cat="Raw")
item("quartz", "Quartz", cat="Raw")
item("raw_bauxite", "Raw Bauxite", cat="Raw")
item("salt", "Rock Salt", cat="Raw")
item("raw_nickel", "Raw Nickel", cat="Raw")
item("raw_tungsten", "Raw Tungsten", cat="Raw")
item("raw_chromium", "Raw Chromium", cat="Raw")
item("graphite", "Graphite", cat="Raw")
item("fluorite", "Fluorite", cat="Raw")
item("raw_zircon", "Raw Zircon", cat="Raw")
item("raw_gold", "Raw Gold", cat="Raw")
item("uranium_ore", "Uranium Ore", cat="Raw", desc="Radioactive. Handle with care.")
item("monazite_sand", "Monazite Sand", cat="Raw")
item("core_crystal", "Core Crystal", stack=8, cat="Raw", desc="A trophy from the mantle edge.")

# materials
item("brick", "Brick", cat="Material")
item("fire_brick", "Fire Brick", cat="Material")
item("iron_ingot", "Iron Ingot", cat="Material")
item("copper_ingot", "Copper Ingot", cat="Material")
item("tin_ingot", "Tin Ingot", cat="Material")
item("bronze_ingot", "Bronze Ingot", cat="Material")
item("lead_ingot", "Lead Ingot", cat="Material")
item("zinc_ingot", "Zinc Ingot", cat="Material")
item("steel_ingot", "Steel Ingot", cat="Material")
item("aluminium_ingot", "Aluminium Ingot", cat="Material")
item("nickel_ingot", "Nickel Ingot", cat="Material")
item("chromium_ingot", "Chromium Ingot", cat="Material")
item("tungsten_ingot", "Tungsten Ingot", cat="Material")
item("gold_ingot", "Gold Ingot", cat="Material")
item("zirconium_ingot", "Zirconium Ingot", cat="Material")
item("stainless_steel", "Stainless Steel", cat="Material")
item("tungsten_carbide", "Tungsten Carbide", cat="Material")
item("glass", "Glass", cat="Material")
item("silicon", "Silicon", cat="Material")
item("cadmium", "Cadmium", cat="Material", desc="Byproduct of zinc refining; control rods.")
item("chlorine", "Chlorine", cat="Material")
item("lye", "Lye", cat="Material")
item("sulfuric_acid", "Sulfuric Acid", cat="Material")
item("fluorine_compound", "Fluorine Compound", cat="Material")
item("graphite_block", "Graphite Block", cat="Material")

# components
item("iron_plate", "Iron Plate", cat="Component")
item("copper_plate", "Copper Plate", cat="Component")
item("steel_plate", "Steel Plate", cat="Component")
item("lead_plate", "Lead Plate", cat="Component")
item("copper_wire", "Copper Wire", cat="Component")
item("copper_pipe", "Copper Pipe", cat="Component")
item("iron_rod", "Iron Rod", cat="Component")
item("steel_rod", "Steel Rod", cat="Component")
item("bronze_gear", "Bronze Gear", cat="Component")
item("steel_gear", "Steel Gear", cat="Component")
item("bearing", "Precision Bearing", cat="Component")
item("turbine_blade", "Turbine Blade", cat="Component")
item("stainless_rotor", "Stainless Rotor", cat="Component")
item("rare_earth_magnet", "Rare-Earth Magnet", cat="Component")
item("circuit_mk1", "Circuit Mk1", cat="Component")
item("circuit_mk2", "Circuit Mk2", cat="Component")
item("zirconium_tubing", "Zirconium Tubing", cat="Component")
item("stainless_casing", "Stainless Casing", cat="Component")
item("lead_shielding", "Lead Shielding", cat="Component")
item("dense_shielding_plate", "Dense Shielding Plate", cat="Component")
item("control_rod", "Control Rod", cat="Component")
item("battery_pack", "Battery Pack", stack=16, cat="Component")
item("empty_canister", "Empty Canister", stack=16, cat="Component")
item("steam_canister", "Steam Canister", stack=1, cat="Component")
item("battery_cell", "Battery Cell", stack=1, cat="Component")
item("drill_head", "Drill Head", stack=1, cat="Component")
item("tungsten_drill_head", "Tungsten Drill Head", stack=1, cat="Component")
item("steel_frame", "Stainless Frame", stack=8, cat="Component")
item("rare_earth_motor", "Rare-Earth Motor", stack=4, cat="Component")
item("conveyor", "Conveyor Belt", cat="Component")
item("rail", "Rail", cat="Component")
item("ladder", "Ladder", cat="Component")
item("oil", "Lamp Oil", stack=16, cat="Component")
item("water", "Water", stack=16, cat="Component", desc="Drawn by a hand pump or electric pump.")
item("methane", "Methane", stack=16, cat="Component")
item("water_bucket", "Water Bucket", stack=8, cat="Component")
item("copper_coil", "Copper Coil", cat="Component")
item("steel_casing", "Steel Casing", cat="Component")
item("oil_canister", "Oil Canister", stack=8, cat="Component")
item("rare_earth_oxide", "Rare-Earth Oxide", cat="Component")
item("thorium_concentrate", "Thorium Concentrate", stack=16, cat="Component")
item("tailings_brick", "Tailings Brick", cat="Component")
item("coolant", "Coolant", stack=16, cat="Component")
item("heat_sink", "Heat Sink", cat="Component")
item("flux", "Limestone Flux", cat="Component")
item("cladding_sleeve", "Cladding Sleeve", cat="Component")
item("magnet_core", "Magnet Core", cat="Component")
item("insulated_cable", "Insulated Cable", cat="Component")


# chemicals / nuclear intermediates
item("crushed_copper_ore", "Crushed Copper Ore", cat="Component")
item("crushed_iron_ore", "Crushed Iron Ore", cat="Component")
item("crushed_lead_ore", "Crushed Lead Ore", cat="Component")
item("crushed_zinc_ore", "Crushed Zinc Ore", cat="Component")
item("crushed_uranium_ore", "Crushed Uranium Ore", cat="Component")
item("washed_copper_ore", "Washed Copper Ore", cat="Component")
item("washed_iron_ore", "Washed Iron Ore", cat="Component")
item("washed_lead_ore", "Washed Lead Ore", cat="Component")
item("washed_zinc_ore", "Washed Zinc Ore", cat="Component")
item("uranium_solution", "Uranium Solution", stack=16, cat="Component")
item("radioactive_tailings", "Radioactive Tailings", stack=16, cat="Component")
item("yellowcake", "Yellowcake", stack=16, cat="Component")
item("uranium_hexafluoride", "UF6 Canister", stack=4, cat="Component")
item("enriched_canister", "Enriched Canister", stack=4, cat="Component")
item("depleted_canister", "Depleted Canister", stack=4, cat="Component")
item("fuel_rod", "Fuel Rod", stack=8, cat="Component")
item("compact_core_cell", "Compact Core Cell", stack=1, cat="Component")
item("spent_fuel_rod", "Spent Fuel Rod", stack=8, cat="Component")
item("spent_core_cell", "Spent Core Cell", stack=1, cat="Component")
item("waste_cask", "Waste Cask", stack=4, cat="Component")

# gear
item("cooling_suit", "Cooling Suit", stack=1, cat="Gear")
item("respirator", "Respirator", stack=1, cat="Gear")
item("hazmat_suit", "Lead-Lined Hazmat Suit", stack=1, cat="Gear")
item("geiger_counter", "Geiger Counter", stack=1, cat="Gear")
item("oil_lamp", "Oil Lamp", stack=1, cat="Gear")
item("headlamp_mk2", "Headlamp Mk2", stack=1, cat="Gear")
item("electric_lamp", "Electric Lamp", stack=8, cat="Block")
item("ore_scanner", "Ore Scanner", stack=1, cat="Gear")

# tools
item("steel_pickaxe", "Steel Pickaxe", stack=1, cat="Tool", tool=("1", "1.0", (1,1), ("None", 0), 0.0))
item("hardened_steel_pickaxe", "Hardened Steel Pickaxe", stack=1, cat="Tool",
     tool=("2", "1.6", (1,1), ("None", 0), 0.0))
item("steam_drill", "Steam Drill", stack=1, cat="Tool",
     tool=("3", "3.0", (1,1), ("Steam", 300), 1.0))
item("tungsten_carbide_drill", "Tungsten-Carbide Drill", stack=1, cat="Tool",
     tool=("4", "5.0", (1,1), ("Battery", 400), 1.0))
item("nuclear_drill", "Nuclear Drill", stack=1, cat="Tool",
     tool=("5", "12.0", (3,3), ("CoreCell", 2000), 1.0))

# blocks as items (placeable)
NON_PLACEABLE = {
    "air", "bedrock", "water", "methane", "grass", "clay", "shale", "granite",
    "basalt", "gneiss", "obsidian", "mantle_rock", "coal_seam", "malachite",
    "cassiterite", "hematite", "galena", "sphalerite", "sulfur_deposit",
    "quartz_vein", "bauxite", "rock_salt", "pentlandite", "wolframite",
    "chromite", "graphite_vein", "fluorite", "zircon", "gold_vein",
    "pitchblende", "monazite", "dense_pitchblende", "core_crystal",
}
PLACEABLE = [b["id"] for b in B if b["id"] not in NON_PLACEABLE
             and b.get("machine") is None]
for pid in PLACEABLE:
    if not any(i["id"] == pid for i in I):
        name = pid.replace("_", " ").title()
        item(pid, name, cat="Block")

# machine blocks as items
for bid, machine in MACHINE_BLOCKS:
    if not any(i["id"] == bid for i in I):
        item(bid, bid.replace("_", " ").replace(" Block", "").title(), stack=8, cat="Block")

print("blocks", len(B), "items", len(I))
json.dump({"blocks": B, "items": I}, open("/tmp/content_stage1.json", "w"), indent=1)
