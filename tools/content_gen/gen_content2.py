#!/usr/bin/env python3
"""Stage 2: machines, recipes, tech tree, layers -> RON files."""
import os, json

HERE = os.path.dirname(__file__)
OUT = os.path.join(HERE, "..", "..", "assets", "data")
data = json.load(open("/tmp/content_stage1.json"))
blocks, items = data["blocks"], data["items"]
item_ids = {i["id"] for i in items}
block_ids = {b["id"] for b in blocks}

# ---------------------------------------------------------------------------
# Machines: (id, name, size, energy, in, out, build_cost, tech, cascade, upgrades, producer, reactor)
# ---------------------------------------------------------------------------
M = []
def machine(id, name, size, energy, ins, outs, cost, tech=None, cascade=None,
            upgrades=0, producer=False, reactor=None):
    M.append(dict(id=id, name=name, size=size, energy=energy, ins=ins, outs=outs,
                  cost=cost, tech=tech, cascade=cascade, upgrades=upgrades,
                  producer=producer, reactor=reactor))

machine("workbench", "Workbench", (1,1,1), "Manual", 9, 9,
        [("timber", 4)], "hand_tools")
machine("storage_crate", "Storage Crate", (1,1,1), "Manual", 27, 27,
        [("timber", 6), ("timber_block", 2)], "hand_tools")
machine("research_desk", "Research Desk", (2,1,1), "Manual", 9, 0,
        [("timber", 8), ("stone_cobble", 4)], "hand_tools")
machine("brick_kiln", "Brick Kiln", (2,2,2), "Fuel(slots: 1, fuel: \"coal\")", 2, 2,
        [("clay_lump", 8), ("stone_cobble", 4)], "masonry")
machine("stone_furnace", "Stone Furnace", (2,2,2), "Fuel(slots: 1, fuel: \"coal\")", 2, 2,
        [("stone_cobble", 12), ("clay_lump", 4)], "smelting")
machine("anvil", "Anvil", (1,1,1), "Manual", 2, 2,
        [("iron_ingot", 8), ("stone_cobble", 4)], "smelting", upgrades=2)
machine("blast_furnace", "Blast Furnace", (2,3,2), "Fuel(slots: 1, fuel: \"coal\")", 3, 2,
        [("fire_brick", 40), ("iron_plate", 12)], "steelmaking", upgrades=2)
machine("hand_pump", "Hand Pump", (1,2,1), "Manual", 0, 1,
        [("copper_plate", 2), ("iron_rod", 2)], "steam_power")
machine("steam_boiler", "Steam Boiler", (2,2,2), "Fuel(slots: 1, fuel: \"coal\")", 1, 0,
        [("steel_plate", 8), ("copper_pipe", 4)], "steam_power", upgrades=2, producer=True)
machine("steam_engine", "Steam Engine", (2,2,2), "Steam(per_sec: 2.0)", 0, 0,
        [("steel_ingot", 6), ("bronze_gear", 2)], "steam_power")
machine("crusher", "Crusher", (2,2,2), "Steam(per_sec: 3.0)", 1, 2,
        [("steel_plate", 10), ("bronze_gear", 4)], "ore_processing", upgrades=2)
machine("ore_washer", "Ore Washer", (2,2,2), "Steam(per_sec: 2.0)", 2, 2,
        [("steel_plate", 8), ("copper_pipe", 6)], "ore_processing", upgrades=2)
machine("mine_lift", "Mine Lift", (2,2,2), "Steam(per_sec: 2.0)", 1, 1,
        [("steel_plate", 12), ("bronze_gear", 4), ("rail", 8)], "logistics")
machine("steam_turbine", "Steam Turbine", (3,3,3), "Steam(per_sec: 4.0)", 0, 0,
        [("steel_plate", 20), ("turbine_blade", 4), ("circuit_mk1", 2)], "electricity",
        upgrades=2, producer=True)
machine("generator", "Generator", (2,2,2), "Fuel(slots: 1, fuel: \"coal\")", 0, 0,
        [("steel_plate", 10), ("copper_wire", 8)], "electricity", upgrades=2, producer=True)
machine("battery_bank", "Battery Bank", (2,2,2), "Manual", 0, 0,
        [("battery_pack", 8), ("lead_plate", 6)], "energy_storage", upgrades=2)
machine("arc_furnace", "Arc Furnace", (2,3,2), "Electric(kw: 30.0)", 3, 3,
        [("steel_plate", 16), ("circuit_mk1", 4), ("graphite", 8)], "arc_smelting", upgrades=2)
machine("electrolysis_cell", "Electrolysis Cell", (2,2,2), "Electric(kw: 40.0)", 2, 3,
        [("steel_plate", 12), ("copper_wire", 12), ("glass", 4)], "electrolysis", upgrades=2)
machine("circuit_assembler", "Circuit Assembler", (2,2,2), "Electric(kw: 20.0)", 3, 2,
        [("steel_plate", 8), ("circuit_mk1", 2), ("copper_wire", 10)], "circuits", upgrades=2)
machine("charging_station", "Charging Station", (1,2,1), "Electric(kw: 10.0)", 2, 1,
        [("steel_plate", 6), ("copper_wire", 6), ("battery_pack", 2)], "energy_storage")
machine("vent_fan", "Vent Fan", (1,1,1), "Electric(kw: 8.0)", 0, 0,
        [("steel_plate", 4), ("circuit_mk1", 1)], "ventilation")
machine("electric_pump", "Electric Pump", (1,2,1), "Electric(kw: 12.0)", 0, 1,
        [("steel_plate", 4), ("copper_pipe", 4), ("circuit_mk1", 1)], "ventilation")
machine("chemical_plant", "Chemical Plant", (3,3,3), "Electric(kw: 50.0)", 3, 3,
        [("stainless_steel", 8), ("glass", 8), ("circuit_mk2", 2)], "chemistry", upgrades=2)
machine("alloy_furnace", "Alloy Furnace", (2,3,2), "Electric(kw: 80.0)", 3, 2,
        [("stainless_steel", 12), ("graphite", 12), ("circuit_mk2", 2)], "alloy_metallurgy", upgrades=2)
machine("machine_shop", "Machine Shop", (3,2,3), "Electric(kw: 60.0)", 3, 3,
        [("stainless_steel", 10), ("bearing", 4), ("circuit_mk2", 2)], "machine_shop", upgrades=2)
machine("rare_earth_separator", "Rare-Earth Separator", (2,2,2), "Electric(kw: 70.0)", 2, 2,
        [("stainless_steel", 8), ("rare_earth_magnet", 2), ("circuit_mk2", 3)], "rare_earths", upgrades=2)
machine("decon_shower", "Decon Shower", (1,2,1), "Electric(kw: 5.0)", 0, 0,
        [("stainless_steel", 4), ("lead_plate", 4), ("circuit_mk2", 1)], "hazard_gear")
machine("leaching_tank", "Leaching Tank", (2,3,2), "Electric(kw: 40.0)", 3, 3,
        [("stainless_steel", 10), ("lead_plate", 8), ("circuit_mk2", 2)], "uranium_milling", upgrades=2)
machine("precipitator", "Precipitator", (2,2,2), "Electric(kw: 30.0)", 2, 2,
        [("stainless_steel", 8), ("lead_plate", 4)], "uranium_milling", upgrades=2)
machine("conversion_plant", "Conversion Plant", (2,2,2), "Electric(kw: 60.0)", 2, 2,
        [("stainless_steel", 8), ("fluorine_compound", 4), ("circuit_mk2", 2)], "enrichment", upgrades=2)
machine("centrifuge", "Centrifuge", (1,3,1), "Electric(kw: 150.0)", 2, 2,
        [("stainless_rotor", 2), ("rare_earth_magnet", 4), ("circuit_mk2", 2)], "enrichment",
        cascade=(4, 16), upgrades=2)
machine("fuel_fabricator", "Fuel Fabricator", (2,2,2), "Electric(kw: 50.0)", 3, 2,
        [("stainless_casing", 6), ("circuit_mk2", 2), ("rare_earth_magnet", 1)], "fuel_fabrication", upgrades=2)
machine("reactor", "Reactor Core", (5,5,5), "Manual", 6, 0,
        [("reactor_casing", 40), ("lead_shielding", 20), ("graphite_block", 16)], "reactor_tech",
        producer=True, reactor=(2000.0, 900.0, 1200.0, 8.0, 12.0))
machine("reactor_console", "Reactor Console", (1,2,1), "Electric(kw: 1.0)", 0, 0,
        [("stainless_steel", 4), ("circuit_mk2", 2), ("glass", 2)], "reactor_tech")
machine("scram_panel", "Scram Panel", (1,1,1), "Electric(kw: 1.0)", 0, 0,
        [("stainless_steel", 2), ("circuit_mk2", 1), ("control_rod", 1)], "reactor_tech")
machine("coolant_pump", "Coolant Pump", (1,2,1), "Electric(kw: 20.0)", 0, 0,
        [("stainless_steel", 4), ("copper_pipe", 6)], "reactor_tech")
machine("heat_exchanger", "Heat Exchanger", (2,2,2), "Electric(kw: 10.0)", 0, 0,
        [("stainless_steel", 6), ("copper_pipe", 8)], "reactor_tech")
machine("cooling_pool", "Cooling Pool", (3,2,3), "Manual", 1, 1,
        [("lead_plate", 10), ("glass", 6), ("stainless_steel", 4)], "waste_handling")
machine("waste_vault", "Waste Vault", (3,3,3), "Manual", 27, 0,
        [("lead_plate", 20), ("dense_shielding_plate", 4)], "waste_handling")

machine_ids = {m["id"] for m in M}

# ---------------------------------------------------------------------------
# Recipes: (id, machine, time, inputs, outputs, tech, hand)
# ---------------------------------------------------------------------------
R = []
def recipe(id, machine, time, ins, outs, tech=None, hand=False, power=None):
    R.append(dict(id=id, machine=machine, time=time, ins=ins, outs=outs,
                  tech=tech, hand=hand, power=power))

# --- Era 0: hand crafting at the workbench --------------------------------
recipe("hand_timber_block", "workbench", 1.0, [("timber", 1)], [("timber_block", 1)], "hand_tools", True)
recipe("hand_timber_support", "workbench", 1.5, [("timber", 2)], [("timber_support", 2)], "hand_tools", True)
recipe("hand_ladder", "workbench", 1.5, [("timber", 3)], [("ladder", 3)], "hand_tools", True)
recipe("hand_scaffold", "workbench", 1.5, [("timber", 2)], [("scaffold", 2)], "hand_tools", True)
recipe("hand_oil_lamp", "workbench", 2.0, [("clay_lump", 2), ("timber", 1)], [("oil_lamp", 1)], "hand_tools", True)
recipe("hand_workbench", "workbench", 2.0, [("timber", 4)], [("workbench", 1)], "hand_tools", True)
recipe("hand_crate", "workbench", 3.0, [("timber", 6), ("timber_block", 2)], [("storage_crate", 1)], "hand_tools", True)
recipe("hand_research_desk", "workbench", 3.0, [("timber", 8), ("stone_cobble", 4)], [("research_desk", 1)], "hand_tools", True)
recipe("hand_gravel_from_cobble", "workbench", 1.0, [("stone_cobble", 1)], [("gravel", 1)], "hand_tools", True)
recipe("hand_lamp_from_gravel", "workbench", 1.0, [("gravel", 1), ("coal", 1)], [("oil", 1)], "hand_tools", True)
recipe("hand_bucket", "workbench", 2.0, [("iron_plate", 2)], [("water_bucket", 1)], "smelting", True)
recipe("hand_bucket_fill", "hand_pump", 1.0, [("water_bucket", 1)], [("water", 4)], "steam_power")

# --- Era 1: masonry and smelting ------------------------------------------
recipe("kiln_brick", "brick_kiln", 4.0, [("clay_lump", 2)], [("brick", 2)], "masonry")
recipe("kiln_fire_brick", "brick_kiln", 6.0, [("brick", 1), ("quartz", 1)], [("fire_brick", 1)], "masonry")
recipe("kiln_glass", "brick_kiln", 5.0, [("sand", 2)], [("glass", 1)], "masonry")
recipe("kiln_brick_block", "brick_kiln", 6.0, [("brick", 4)], [("brick_block", 1)], "masonry")
recipe("furnace_stone", "stone_furnace", 3.0, [("stone_cobble", 2)], [("stone", 1)], "masonry")
recipe("smelt_copper", "stone_furnace", 6.0, [("raw_copper", 1)], [("copper_ingot", 1)], "smelting")
recipe("smelt_tin", "stone_furnace", 6.0, [("raw_tin", 1)], [("tin_ingot", 1)], "smelting")
recipe("smelt_iron", "stone_furnace", 7.0, [("raw_iron", 1)], [("iron_ingot", 1)], "smelting")
recipe("smelt_lead", "stone_furnace", 7.0, [("raw_lead", 1)], [("lead_ingot", 1)], "smelting")
recipe("smelt_zinc", "stone_furnace", 7.0, [("raw_zinc", 2)], [("zinc_ingot", 1), ("cadmium", 1)], "smelting")
recipe("smelt_bronze", "stone_furnace", 8.0, [("copper_ingot", 1), ("tin_ingot", 1)], [("bronze_ingot", 2)], "smelting")
recipe("smelt_glass", "stone_furnace", 6.0, [("sand", 3)], [("glass", 1)], "smelting")
recipe("furnace_acid", "stone_furnace", 12.0, [("sulfur", 2), ("water", 1)], [("sulfuric_acid", 1)], "smelting")

recipe("anvil_iron_plate", "anvil", 3.0, [("iron_ingot", 1)], [("iron_plate", 1)], "smelting")
recipe("anvil_copper_plate", "anvil", 3.0, [("copper_ingot", 1)], [("copper_plate", 1)], "smelting")
recipe("anvil_lead_plate", "anvil", 3.0, [("lead_ingot", 1)], [("lead_plate", 1)], "smelting")
recipe("anvil_copper_wire", "anvil", 2.0, [("copper_ingot", 1)], [("copper_wire", 2)], "smelting")
recipe("anvil_iron_rod", "anvil", 2.0, [("iron_ingot", 1)], [("iron_rod", 2)], "smelting")
recipe("anvil_bronze_gear", "anvil", 4.0, [("bronze_ingot", 2)], [("bronze_gear", 1)], "smelting")
recipe("anvil_bearing", "anvil", 5.0, [("steel_ingot", 2), ("bronze_ingot", 1)], [("bearing", 1)], "steelmaking")
recipe("anvil_steel_rod", "anvil", 3.0, [("steel_ingot", 1)], [("steel_rod", 2)], "steelmaking")
recipe("anvil_steel_gear", "anvil", 5.0, [("steel_ingot", 2)], [("steel_gear", 1)], "steelmaking")
recipe("anvil_rail", "anvil", 4.0, [("iron_ingot", 2)], [("rail", 8)], "steelmaking")
recipe("anvil_ladder_metal", "anvil", 3.0, [("iron_rod", 2)], [("ladder", 2)], "steelmaking")
recipe("anvil_steel_support", "anvil", 4.0, [("steel_plate", 2)], [("steel_support", 1)], "steelmaking")

recipe("blast_steel", "blast_furnace", 12.0, [("iron_ingot", 2), ("coal", 1), ("limestone", 1)], [("steel_ingot", 2)], "steelmaking")
recipe("blast_steel_plate", "blast_furnace", 14.0, [("steel_ingot", 1)], [("steel_plate", 1)], "steelmaking")
recipe("blast_steel_block", "blast_furnace", 20.0, [("steel_plate", 2)], [("steel_block", 1)], "steelmaking")
recipe("blast_fire_brick", "blast_furnace", 10.0, [("clay_lump", 3), ("quartz", 1)], [("fire_brick", 2)], "steelmaking")
recipe("anvil_hardened_pickaxe", "anvil", 20.0, [("steel_ingot", 3), ("iron_rod", 2), ("bronze_gear", 1)], [("hardened_steel_pickaxe", 1)], "steelmaking")

# --- Era 2: steam and logistics -------------------------------------------
recipe("pump_water", "hand_pump", 2.0, [("water_bucket", 1)], [("water", 2)], "steam_power")
recipe("boiler_steam", "steam_boiler", 2.0, [("water", 1)], [("empty_canister", 1)], "steam_power")
recipe("boiler_refill_steam", "steam_boiler", 4.0, [("empty_canister", 1)], [("steam_canister", 1)], "steam_power")
recipe("crusher_copper", "crusher", 5.0, [("raw_copper", 1)], [("crushed_copper_ore", 1)], "ore_processing")
recipe("crusher_iron", "crusher", 5.0, [("raw_iron", 1)], [("crushed_iron_ore", 1)], "ore_processing")
recipe("crusher_lead", "crusher", 5.0, [("raw_lead", 1)], [("crushed_lead_ore", 1)], "ore_processing")
recipe("crusher_zinc", "crusher", 5.0, [("raw_zinc", 1)], [("crushed_zinc_ore", 1)], "ore_processing")
recipe("crusher_uranium", "crusher", 8.0, [("uranium_ore", 1)], [("crushed_uranium_ore", 1)], "uranium_milling")
recipe("crusher_graphite", "crusher", 6.0, [("graphite", 2)], [("graphite_block", 1)], "arc_smelting")
recipe("washer_copper", "ore_washer", 6.0, [("crushed_copper_ore", 1), ("water", 1)], [("washed_copper_ore", 1)], "ore_processing")
recipe("washer_iron", "ore_washer", 6.0, [("crushed_iron_ore", 1), ("water", 1)], [("washed_iron_ore", 1)], "ore_processing")
recipe("washer_lead", "ore_washer", 6.0, [("crushed_lead_ore", 1), ("water", 1)], [("washed_lead_ore", 1)], "ore_processing")
recipe("washer_zinc", "ore_washer", 6.0, [("crushed_zinc_ore", 1), ("water", 1)], [("washed_zinc_ore", 1)], "ore_processing")
recipe("smelt_washed_copper", "stone_furnace", 6.0, [("washed_copper_ore", 1)], [("copper_ingot", 2)], "ore_processing")
recipe("smelt_washed_iron", "stone_furnace", 7.0, [("washed_iron_ore", 1)], [("iron_ingot", 2)], "ore_processing")
recipe("smelt_washed_lead", "stone_furnace", 7.0, [("washed_lead_ore", 1)], [("lead_ingot", 2)], "ore_processing")
recipe("smelt_washed_zinc", "stone_furnace", 7.0, [("washed_zinc_ore", 1)], [("zinc_ingot", 1), ("cadmium", 1)], "ore_processing")
recipe("smelt_crushed_copper", "stone_furnace", 6.0, [("crushed_copper_ore", 2)], [("copper_ingot", 3)], "ore_processing")
recipe("smelt_crushed_iron", "stone_furnace", 7.0, [("crushed_iron_ore", 2)], [("iron_ingot", 3)], "ore_processing")
recipe("anvil_copper_pipe", "anvil", 4.0, [("copper_ingot", 2)], [("copper_pipe", 2)], "steam_power")
recipe("anvil_empty_canister", "anvil", 3.0, [("steel_plate", 1), ("copper_pipe", 1)], [("empty_canister", 1)], "steam_power")
recipe("anvil_drill_head", "anvil", 8.0, [("steel_plate", 3), ("bronze_gear", 1)], [("drill_head", 1)], "steam_drill")
recipe("anvil_steam_drill", "anvil", 25.0, [("steel_ingot", 4), ("bronze_gear", 2), ("copper_pipe", 4), ("drill_head", 1)], [("steam_drill", 1)], "steam_drill")
recipe("hand_conveyor", "workbench", 2.0, [("timber", 2), ("iron_plate", 1)], [("conveyor", 2)], "logistics", True)
recipe("hand_cable", "workbench", 1.0, [("copper_wire", 2)], [("cable_block", 1)], "electricity", True)
recipe("hand_flux", "workbench", 1.0, [("limestone", 1)], [("flux", 1)], "smelting", True)
recipe("hand_oil_canister", "workbench", 2.0, [("oil", 4), ("empty_canister", 1)], [("oil_canister", 1)], "hand_tools", True)
recipe("anvil_copper_coil", "anvil", 4.0, [("copper_wire", 4)], [("copper_coil", 1)], "electricity")
recipe("anvil_steel_casing", "anvil", 5.0, [("steel_plate", 3)], [("steel_casing", 1)], "steelmaking")
recipe("anvil_insulated_cable", "anvil", 4.0, [("copper_wire", 2), ("gravel", 1)], [("insulated_cable", 2)], "electricity")
recipe("anvil_heat_sink", "anvil", 6.0, [("copper_plate", 3), ("aluminium_ingot", 1)], [("heat_sink", 1)], "machine_shop")
recipe("shop_cladding_sleeve", "machine_shop", 8.0, [("zirconium_tubing", 2)], [("cladding_sleeve", 1)], "fuel_fabrication")
recipe("shop_magnet_core", "machine_shop", 10.0, [("rare_earth_magnet", 2), ("steel_casing", 1)], [("magnet_core", 1)], "rare_earths")
recipe("chem_coolant", "chemical_plant", 10.0, [("water", 2), ("lye", 1)], [("coolant", 2)], "chemistry")
recipe("chem_rare_earth_oxide", "chemical_plant", 14.0, [("monazite_sand", 2), ("sulfuric_acid", 1)], [("rare_earth_oxide", 1), ("thorium_concentrate", 1)], "rare_earths")
recipe("kiln_tailings_brick", "brick_kiln", 8.0, [("radioactive_tailings", 1), ("clay_lump", 2)], [("tailings_brick", 1)], "waste_handling")
recipe("fabricate_fuel_rod_clad", "fuel_fabricator", 24.0, [("enriched_canister", 1), ("cladding_sleeve", 1)], [("fuel_rod", 1)], "fuel_fabrication")
recipe("hand_pipe", "workbench", 1.0, [("copper_pipe", 2)], [("pipe_block", 1)], "steam_power", True)

# --- Era 3: electricity ----------------------------------------------------
recipe("turbine_power", "steam_turbine", 2.0, [("steam_canister", 1)], [("empty_canister", 1)], "electricity")
recipe("generator_power", "generator", 2.0, [("coal", 1)], [("gravel", 1)], "electricity")
recipe("arc_silicon", "arc_furnace", 8.0, [("quartz", 2)], [("silicon", 1)], "arc_smelting")
recipe("arc_steel", "arc_furnace", 6.0, [("iron_ingot", 2), ("coal", 1)], [("steel_ingot", 2)], "arc_smelting")
recipe("arc_aluminium", "arc_furnace", 10.0, [("raw_bauxite", 2)], [("aluminium_ingot", 1)], "arc_smelting")
recipe("arc_glass", "arc_furnace", 5.0, [("sand", 3)], [("glass", 2)], "arc_smelting")
recipe("electro_aluminium", "electrolysis_cell", 12.0, [("raw_bauxite", 3)], [("aluminium_ingot", 2)], "electrolysis")
recipe("electro_salt", "electrolysis_cell", 8.0, [("salt", 2), ("water", 1)], [("chlorine", 1), ("lye", 1)], "electrolysis")
recipe("electro_acid", "electrolysis_cell", 10.0, [("sulfur", 2), ("water", 1)], [("sulfuric_acid", 2)], "electrolysis")
recipe("circuit_mk1", "circuit_assembler", 8.0, [("silicon", 1), ("copper_wire", 2)], [("circuit_mk1", 1)], "circuits")
recipe("circuit_mk2", "circuit_assembler", 14.0, [("silicon", 2), ("circuit_mk1", 1), ("gold_ingot", 1)], [("circuit_mk2", 1)], "circuits")
recipe("assembler_battery", "circuit_assembler", 10.0, [("lead_plate", 2), ("sulfuric_acid", 1), ("copper_plate", 1)], [("battery_pack", 1)], "energy_storage")
recipe("assembler_battery_cell", "circuit_assembler", 8.0, [("battery_pack", 1), ("circuit_mk1", 1)], [("battery_cell", 1)], "energy_storage")
recipe("charge_battery_cell", "charging_station", 6.0, [("battery_cell", 1)], [("battery_cell", 1)], "energy_storage")
recipe("anvil_headlamp_mk2", "anvil", 8.0, [("steel_plate", 2), ("circuit_mk1", 1), ("battery_cell", 1)], [("headlamp_mk2", 1)], "circuits")
recipe("anvil_ore_scanner", "anvil", 12.0, [("steel_plate", 2), ("circuit_mk1", 2), ("glass", 2)], [("ore_scanner", 1)], "ore_scanner")
recipe("assembler_electric_lamp", "circuit_assembler", 4.0, [("glass", 1), ("copper_wire", 1)], [("electric_lamp", 2)], "circuits")

# --- Era 4: chemistry and deep metallurgy ---------------------------------
recipe("chem_acid", "chemical_plant", 8.0, [("sulfur", 2), ("water", 1)], [("sulfuric_acid", 3)], "chemistry")
recipe("chem_fluorine", "chemical_plant", 12.0, [("fluorite", 2), ("sulfuric_acid", 1)], [("fluorine_compound", 1)], "chemistry")
recipe("chem_lye", "chemical_plant", 8.0, [("salt", 2), ("water", 1)], [("lye", 2), ("chlorine", 1)], "chemistry")
recipe("alloy_stainless", "alloy_furnace", 15.0, [("steel_ingot", 2), ("nickel_ingot", 1), ("chromium_ingot", 1)], [("stainless_steel", 2)], "alloy_metallurgy")
recipe("alloy_tungsten_carbide", "alloy_furnace", 18.0, [("tungsten_ingot", 1), ("graphite", 2)], [("tungsten_carbide", 1)], "alloy_metallurgy")
recipe("alloy_nickel", "alloy_furnace", 8.0, [("raw_nickel", 1)], [("nickel_ingot", 1)], "alloy_metallurgy")
recipe("alloy_chromium", "alloy_furnace", 8.0, [("raw_chromium", 1)], [("chromium_ingot", 1)], "alloy_metallurgy")
recipe("alloy_tungsten", "alloy_furnace", 12.0, [("raw_tungsten", 2)], [("tungsten_ingot", 1)], "alloy_metallurgy")
recipe("alloy_gold", "alloy_furnace", 8.0, [("raw_gold", 1)], [("gold_ingot", 1)], "alloy_metallurgy")
recipe("alloy_zirconium", "alloy_furnace", 12.0, [("raw_zircon", 2)], [("zirconium_ingot", 1)], "alloy_metallurgy")
recipe("shop_bearing", "machine_shop", 10.0, [("steel_ingot", 2), ("bronze_ingot", 1)], [("bearing", 2)], "machine_shop")
recipe("shop_turbine_blade", "machine_shop", 12.0, [("stainless_steel", 1), ("bearing", 1)], [("turbine_blade", 2)], "machine_shop")
recipe("shop_stainless_rotor", "machine_shop", 15.0, [("stainless_steel", 2), ("bearing", 2)], [("stainless_rotor", 1)], "machine_shop")
recipe("shop_stainless_casing", "machine_shop", 10.0, [("stainless_steel", 2)], [("stainless_casing", 1)], "machine_shop")
recipe("shop_steel_frame", "machine_shop", 12.0, [("stainless_steel", 3), ("steel_rod", 2)], [("steel_frame", 1)], "machine_shop")
recipe("shop_rare_earth_motor", "machine_shop", 18.0, [("rare_earth_magnet", 2), ("stainless_rotor", 1), ("copper_wire", 6)], [("rare_earth_motor", 1)], "rare_earths")
recipe("shop_tungsten_drill_head", "machine_shop", 20.0, [("tungsten_carbide", 2), ("bearing", 2), ("steel_plate", 2)], [("tungsten_drill_head", 1)], "tungsten_drill")
recipe("shop_lead_shielding", "machine_shop", 8.0, [("lead_plate", 3)], [("lead_shielding", 1)], "machine_shop")
recipe("shop_zirconium_tubing", "machine_shop", 10.0, [("zirconium_ingot", 1)], [("zirconium_tubing", 2)], "machine_shop")
recipe("shop_control_rod", "machine_shop", 12.0, [("cadmium", 2), ("stainless_steel", 1)], [("control_rod", 1)], "reactor_tech")
recipe("sep_monazite", "rare_earth_separator", 20.0, [("monazite_sand", 3), ("sulfuric_acid", 1)], [("rare_earth_magnet", 1)], "rare_earths")
recipe("shop_tungsten_drill", "machine_shop", 30.0, [("tungsten_carbide", 3), ("steel_rod", 2), ("battery_cell", 2), ("steel_gear", 2), ("drill_head", 1)], [("tungsten_carbide_drill", 1)], "tungsten_drill")
recipe("shop_cooling_suit", "machine_shop", 20.0, [("stainless_steel", 2), ("copper_pipe", 4), ("circuit_mk1", 1)], [("cooling_suit", 1)], "hazard_gear")
recipe("shop_respirator", "machine_shop", 15.0, [("stainless_steel", 1), ("glass", 2), ("circuit_mk1", 1)], [("respirator", 1)], "hazard_gear")
recipe("shop_hazmat_suit", "machine_shop", 25.0, [("lead_shielding", 4), ("stainless_steel", 2), ("circuit_mk2", 1)], [("hazmat_suit", 1)], "hazard_gear")
recipe("shop_geiger_counter", "machine_shop", 15.0, [("circuit_mk1", 2), ("copper_wire", 4), ("glass", 1)], [("geiger_counter", 1)], "hazard_gear")

# --- Era 5: nuclear -------------------------------------------------------
recipe("leach_uranium", "leaching_tank", 20.0, [("crushed_uranium_ore", 2), ("sulfuric_acid", 1)], [("uranium_solution", 1), ("radioactive_tailings", 1)], "uranium_milling")
recipe("precipitate_yellowcake", "precipitator", 18.0, [("uranium_solution", 1), ("lye", 1)], [("yellowcake", 1)], "uranium_milling")
recipe("convert_uf6", "conversion_plant", 25.0, [("yellowcake", 1), ("fluorine_compound", 1)], [("uranium_hexafluoride", 1)], "enrichment")
recipe("centrifuge_enrich", "centrifuge", 30.0, [("uranium_hexafluoride", 1)], [("enriched_canister", 1), ("depleted_canister", 1)], "enrichment")
recipe("fabricate_fuel_rod", "fuel_fabricator", 22.0, [("enriched_canister", 1), ("zirconium_tubing", 2)], [("fuel_rod", 1)], "fuel_fabrication")
recipe("fabricate_core_cell", "fuel_fabricator", 35.0, [("enriched_canister", 1), ("stainless_casing", 1), ("lead_shielding", 2)], [("compact_core_cell", 1)], "fuel_fabrication")
recipe("fabricate_dense_shield", "fuel_fabricator", 25.0, [("depleted_canister", 1), ("lead_plate", 2)], [("dense_shielding_plate", 1)], "fuel_fabrication")
recipe("shop_nuclear_drill", "machine_shop", 60.0, [("compact_core_cell", 1), ("tungsten_drill_head", 1), ("steel_frame", 1), ("rare_earth_motor", 1), ("lead_shielding", 2)], [("nuclear_drill", 1)], "nuclear_drill")
recipe("pool_spent_fuel", "cooling_pool", 60.0, [("spent_fuel_rod", 1)], [("waste_cask", 1)], "waste_handling")
recipe("pool_spent_cell", "cooling_pool", 90.0, [("spent_core_cell", 1)], [("waste_cask", 2)], "waste_handling")
recipe("reactor_graphite_block", "machine_shop", 10.0, [("graphite", 4)], [("graphite_block", 1)], "reactor_tech")
recipe("pool_burn_fuel", "cooling_pool", 30.0, [("fuel_rod", 1), ("uranium_ore", 1)], [("spent_fuel_rod", 1)], "waste_handling")
recipe("pool_burn_cell", "cooling_pool", 45.0, [("compact_core_cell", 1), ("uranium_ore", 2)], [("spent_core_cell", 1)], "waste_handling")
recipe("chem_methane", "chemical_plant", 6.0, [("coal", 2), ("water", 1)], [("methane", 1)], "chemistry")

# ---------------------------------------------------------------------------
# Placeable blocks: every block item can be placed back as its own block.
# ---------------------------------------------------------------------------
PLACE_COST = {
    "stone": "stone_cobble", "brick_block": "brick", "fire_brick_block": "fire_brick",
    "glass_block": "glass", "steel_block": "steel_plate", "lead_block": "lead_plate",
    "dense_shielding": "dense_shielding_plate", "graphite_moderator": "graphite_block",
    "reactor_casing": "stainless_casing", "rail_block": "rail", "conveyor_block": "conveyor",
    "ladder": None, "timber_support": "timber", "scaffold": "timber",
}
TECH_FOR_PLACE = {
    "stone": "masonry", "brick_block": "masonry", "fire_brick_block": "steelmaking",
    "glass_block": "masonry", "steel_block": "steelmaking", "lead_block": "smelting",
    "dense_shielding": "waste_handling", "graphite_moderator": "reactor_tech",
    "reactor_casing": "reactor_tech", "rail_block": "logistics", "conveyor_block": "logistics",
    "ladder": "hand_tools", "timber_support": "hand_tools", "scaffold": "hand_tools",
    "timber_block": "hand_tools", "steel_support": "steelmaking", "electric_lamp": "circuits",
}
for bid in sorted(PLACE_COST):
    src = PLACE_COST[bid] or bid
    if bid == "timber_support":
        ins, outs = [("timber", 1)], [("timber_support", 1)]
    elif bid == "steel_support":
        ins, outs = [("steel_plate", 1)], [("steel_support", 1)]
    elif bid == "scaffold":
        ins, outs = [("timber", 1)], [("scaffold", 1)]
    elif bid == "electric_lamp":
        continue
    elif bid == "ladder":
        ins, outs = [("timber", 1)], [("ladder", 1)]
    else:
        ins, outs = [(src, 1)], [(bid, 1)]
    recipe("place_" + bid, "workbench", 1.0, ins, outs, TECH_FOR_PLACE[bid], True)

# ---------------------------------------------------------------------------
# Tech tree
# ---------------------------------------------------------------------------
T = []
def tech(id, name, era, parents, cost, unlocks, desc):
    T.append(dict(id=id, name=name, era=era, parents=parents, cost=cost,
                  unlocks=unlocks, desc=desc))

tech("hand_tools", "Hand Tools", 0, [], [("timber", 5)],
     ["workbench", "storage_crate", "research_desk", "hand_timber_block", "hand_ladder",
      "hand_oil_lamp", "hand_research_desk"],
     "Set up the camp: workbench, crates and light.")
tech("masonry", "Masonry", 1, ["hand_tools"], [("clay_lump", 10), ("timber", 6)],
     ["brick_kiln", "kiln_brick", "kiln_glass", "kiln_fire_brick", "brick_block"],
     "Fire clay into bricks and glass.")
tech("smelting", "Smelting", 1, ["masonry"], [("coal", 10), ("raw_copper", 6), ("raw_iron", 6)],
     ["stone_furnace", "anvil", "smelt_copper", "smelt_iron", "smelt_tin", "smelt_bronze",
      "anvil_iron_plate", "anvil_copper_wire", "anvil_iron_rod"],
     "Smelt ore into ingots and hammer them into parts.")
tech("steelmaking", "Steelmaking", 1, ["smelting"], [("iron_plate", 12), ("fire_brick", 8), ("limestone", 8)],
     ["blast_furnace", "blast_steel", "blast_steel_plate", "anvil_hardened_pickaxe"],
     "Steel opens the Granite Depths.")
tech("steam_power", "Steam Power", 2, ["steelmaking"],
     [("steel_plate", 20), ("copper_pipe", 10), ("bronze_gear", 5)],
     ["hand_pump", "steam_boiler", "steam_engine", "pipe_block", "anvil_copper_pipe"],
     "Boil water, pipe the steam, drive machines.")
tech("ore_processing", "Ore Processing", 2, ["steam_power"],
     [("steel_plate", 14), ("bronze_gear", 6)],
     ["crusher", "ore_washer", "crusher_iron", "washer_iron", "smelt_washed_iron"],
     "Crush and wash ore for up to double yield.")
tech("logistics", "Logistics", 2, ["steam_power"],
     [("steel_plate", 10), ("rail", 16), ("conveyor", 4)],
     ["mine_lift", "hand_conveyor"],
     "Rails, conveyors and lifts move the ore home.")
tech("steam_drill", "Steam Drill", 2, ["steam_power", "ore_processing"],
     [("steel_ingot", 8), ("copper_pipe", 6), ("bronze_gear", 4)],
     ["anvil_drill_head", "anvil_steam_drill", "anvil_empty_canister"],
     "A drill that runs on canisters filled at the boiler.")
tech("electricity", "Electricity", 3, ["steam_power", "steelmaking"],
     [("copper_wire", 20), ("steel_plate", 16), ("turbine_blade", 2)],
     ["steam_turbine", "generator", "cable_block", "hand_cable"],
     "Turn steam into power and run it down cables.")
tech("arc_smelting", "Arc Smelting", 3, ["electricity"],
     [("steel_plate", 18), ("graphite", 12), ("copper_wire", 16)],
     ["arc_furnace", "arc_silicon", "arc_steel", "arc_aluminium"],
     "Electric furnaces melt faster and refine silicon.")
tech("electrolysis", "Electrolysis", 3, ["electricity"],
     [("steel_plate", 14), ("glass", 6), ("salt", 10)],
     ["electrolysis_cell", "electro_aluminium", "electro_salt", "electro_acid"],
     "Split salt and ore with current.")
tech("circuits", "Circuits", 3, ["electricity", "arc_smelting"],
     [("silicon", 6), ("copper_wire", 12)],
     ["circuit_assembler", "circuit_mk1", "anvil_headlamp_mk2"],
     "Silicon and wire become control circuits.")
tech("energy_storage", "Energy Storage", 3, ["electricity", "circuits"],
     [("lead_plate", 10), ("sulfuric_acid", 6), ("circuit_mk1", 2)],
     ["battery_bank", "charging_station", "assembler_battery", "assembler_battery_cell"],
     "Batteries smooth the grid and power tools.")
tech("ventilation", "Ventilation", 3, ["electricity", "circuits"],
     [("steel_plate", 8), ("circuit_mk1", 2)],
     ["vent_fan", "electric_pump"],
     "Clear gas pockets and pump out flooded tunnels.")
tech("ore_scanner", "Ore Scanner", 3, ["circuits"],
     [("circuit_mk1", 4), ("glass", 4)],
     ["anvil_ore_scanner"],
     "Reveals ore within sixteen blocks.")
tech("chemistry", "Chemistry", 4, ["circuits", "electrolysis"],
     [("sulfuric_acid", 8), ("fluorite", 4), ("glass", 8)],
     ["chemical_plant", "chem_acid", "chem_fluorine", "chem_lye"],
     "Acids and fluorine compounds at scale.")
tech("alloy_metallurgy", "Deep Metallurgy", 4, ["steelmaking", "chemistry"],
     [("nickel_ingot", 6), ("chromium_ingot", 6), ("graphite", 8)],
     ["alloy_furnace", "alloy_stainless", "alloy_tungsten_carbide", "alloy_nickel"],
     "Stainless steel and tungsten carbide.")
tech("machine_shop", "Machine Shop", 4, ["alloy_metallurgy", "circuits"],
     [("stainless_steel", 10), ("bearing", 4)],
     ["machine_shop", "shop_bearing", "shop_turbine_blade", "shop_stainless_rotor"],
     "Precision parts: bearings, rotors, casings.")
tech("rare_earths", "Rare Earths", 4, ["machine_shop"],
     [("monazite_sand", 8), ("sulfuric_acid", 4)],
     ["rare_earth_separator", "sep_monazite", "shop_rare_earth_motor"],
     "Monazite becomes magnets for motors and centrifuges.")
tech("hazard_gear", "Hazard Gear", 4, ["machine_shop", "chemistry"],
     [("stainless_steel", 8), ("lead_plate", 8), ("circuit_mk1", 2)],
     ["decon_shower", "shop_cooling_suit", "shop_respirator", "shop_hazmat_suit", "shop_geiger_counter"],
     "Survive heat, gas and radiation in the Deep Crust.")
tech("tungsten_drill", "Tungsten-Carbide Drill", 4, ["machine_shop", "hazard_gear"],
     [("tungsten_carbide", 6), ("battery_cell", 4), ("steel_rod", 4)],
     ["shop_tungsten_drill_head", "shop_tungsten_drill"],
     "A battery drill that bites into layer five.")
tech("uranium_milling", "Uranium Milling", 5, ["tungsten_drill", "chemistry"],
     [("crushed_uranium_ore", 4), ("sulfuric_acid", 8)],
     ["leaching_tank", "precipitator", "leach_uranium", "precipitate_yellowcake"],
     "Crush, leach and precipitate uranium into yellowcake.")
tech("enrichment", "Enrichment", 5, ["uranium_milling", "rare_earths"],
     [("yellowcake", 2), ("fluorine_compound", 4), ("stainless_rotor", 2)],
     ["conversion_plant", "centrifuge", "convert_uf6", "centrifuge_enrich"],
     "Convert to UF6 and enrich it in a centrifuge cascade.")
tech("fuel_fabrication", "Fuel Fabrication", 5, ["enrichment"],
     [("enriched_canister", 2), ("zirconium_tubing", 4)],
     ["fuel_fabricator", "fabricate_fuel_rod", "fabricate_core_cell", "fabricate_dense_shield"],
     "Enriched canisters become fuel rods and core cells.")
tech("reactor_tech", "Reactor", 5, ["fuel_fabrication", "alloy_metallurgy"],
     [("fuel_rod", 4), ("control_rod", 2), ("graphite_block", 8), ("stainless_casing", 4)],
     ["reactor", "reactor_console", "scram_panel", "coolant_pump", "heat_exchanger", "shop_control_rod"],
     "A 5x5x5 reactor producing twenty times the coal grid.")
tech("nuclear_drill", "Nuclear Drill", 5, ["fuel_fabrication", "tungsten_drill"],
     [("compact_core_cell", 1), ("tungsten_drill_head", 1), ("rare_earth_motor", 1)],
     ["shop_nuclear_drill"],
     "Mines a 3x3 face per action, powered by a core cell.")
tech("waste_handling", "Waste Handling", 5, ["reactor_tech"],
     [("waste_cask", 2), ("dense_shielding_plate", 2)],
     ["cooling_pool", "waste_vault", "pool_spent_fuel"],
     "Cool spent fuel and lock it away.")

# ---------------------------------------------------------------------------
# Layers
# ---------------------------------------------------------------------------
L = [
 dict(id="topsoil", name="Topsoil", depth=(0,16), base_block="dirt",
      mix=[("grass",0.05),("gravel",0.10),("clay",0.06),("sand",0.08)],
      ores=[dict(block="coal_seam",frequency=0.06,vein_size=(4,12)),
            dict(block="rock_salt",frequency=0.005,vein_size=(4,8))],
      hazards=[], ambient_temp=16.0, cave_density=1.2),
 dict(id="shale_beds", name="Shale Beds", depth=(16,64), base_block="stone",
      mix=[("limestone",0.18),("shale",0.20),("dirt",0.02)],
      ores=[dict(block="coal_seam",frequency=0.05,vein_size=(6,24)),
            dict(block="malachite",frequency=0.030,vein_size=(4,16)),
            dict(block="cassiterite",frequency=0.018,vein_size=(4,12)),
            dict(block="hematite",frequency=0.035,vein_size=(6,20))],
      hazards=[], ambient_temp=19.0, cave_density=1.0),
 dict(id="granite_depths", name="Granite Depths", depth=(64,160), base_block="granite",
      mix=[("stone",0.08)],
      ores=[dict(block="hematite",frequency=0.030,vein_size=(6,20)),
            dict(block="galena",frequency=0.018,vein_size=(4,14)),
            dict(block="sphalerite",frequency=0.018,vein_size=(4,14)),
            dict(block="sulfur_deposit",frequency=0.020,vein_size=(4,16)),
            dict(block="quartz_vein",frequency=0.020,vein_size=(4,12)),
            dict(block="bauxite",frequency=0.016,vein_size=(4,14)),
            dict(block="rock_salt",frequency=0.030,vein_size=(6,18))],
      hazards=[dict(kind="flooding",pocket_chance=0.002,max_volume=512)],
      ambient_temp=24.0, cave_density=0.9),
 dict(id="basalt_furnace", name="Basalt Furnace", depth=(160,288), base_block="basalt",
      mix=[("granite",0.10)],
      ores=[dict(block="pentlandite",frequency=0.018,vein_size=(4,12)),
            dict(block="wolframite",frequency=0.008,vein_size=(3,8)),
            dict(block="chromite",frequency=0.016,vein_size=(4,12)),
            dict(block="graphite_vein",frequency=0.018,vein_size=(4,14)),
            dict(block="fluorite",frequency=0.008,vein_size=(3,8)),
            dict(block="zircon",frequency=0.007,vein_size=(3,8)),
            dict(block="gold_vein",frequency=0.006,vein_size=(3,8)),
            dict(block="sulfur_deposit",frequency=0.012,vein_size=(4,12))],
      hazards=[dict(kind="heat",per_sec=0.35),dict(kind="gas",pocket_chance=0.003,oxygen_per_sec=12.0)],
      ambient_temp=38.0, cave_density=0.8),
 dict(id="deep_crust", name="Deep Crust", depth=(288,448), base_block="gneiss",
      mix=[("obsidian",0.12),("basalt",0.10)],
      ores=[dict(block="pitchblende",frequency=0.007,vein_size=(3,9)),
            dict(block="monazite",frequency=0.012,vein_size=(4,10)),
            dict(block="gold_vein",frequency=0.006,vein_size=(3,8))],
      hazards=[dict(kind="radiation",per_sec=0.25),dict(kind="cave_in",max_span=7),
               dict(kind="heat",per_sec=0.8),dict(kind="gas",pocket_chance=0.004,oxygen_per_sec=16.0)],
      ambient_temp=62.0, cave_density=0.7),
 dict(id="mantle_edge", name="Mantle Edge", depth=(448,512), base_block="mantle_rock",
      mix=[("obsidian",0.15)],
      ores=[dict(block="dense_pitchblende",frequency=0.014,vein_size=(4,12)),
            dict(block="core_crystal",frequency=0.003,vein_size=(2,4))],
      hazards=[dict(kind="radiation",per_sec=1.2),dict(kind="heat",per_sec=2.0),
               dict(kind="cave_in",max_span=7)],
      ambient_temp=110.0, cave_density=0.6),
]

json.dump({"machines": M, "recipes": R, "tech": T, "layers": L},
          open("/tmp/content_stage2.json", "w"), indent=1)
print("machines", len(M), "recipes", len(R), "tech", len(T), "layers", len(L))

# ---------------------------------------------------------------------------
# Cross checks
# ---------------------------------------------------------------------------
errs = []
for m in M:
    for it, _ in m["cost"]:
        if it not in item_ids:
            errs.append(f"machine {m['id']} cost unknown item {it}")
for r in R:
    if r["machine"] != "hand" and r["machine"] not in machine_ids:
        errs.append(f"recipe {r['id']} unknown machine {r['machine']}")
    for it, _ in r["ins"] + r["outs"]:
        if it not in item_ids:
            errs.append(f"recipe {r['id']} unknown item {it}")
for t in T:
    for p in t["parents"]:
        if p not in {x["id"] for x in T}:
            errs.append(f"tech {t['id']} missing parent {p}")
    for it, _ in t["cost"]:
        if it not in item_ids:
            errs.append(f"tech {t['id']} unknown cost item {it}")
    for u in t["unlocks"]:
        if u not in machine_ids and u not in {r["id"] for r in R} and u not in block_ids and u not in item_ids:
            errs.append(f"tech {t['id']} unlocks unknown {u}")
for l in L:
    if l["base_block"] not in block_ids:
        errs.append(f"layer {l['id']} unknown base {l['base_block']}")
    for o in l["ores"]:
        if o["block"] not in block_ids:
            errs.append(f"layer {l['id']} unknown ore {o['block']}")
print("ERRORS:", len(errs))
for e in errs:
    print(" -", e)
