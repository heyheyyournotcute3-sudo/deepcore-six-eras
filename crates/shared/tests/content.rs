//! Integration tests that load the real content files from `assets/data`.

use std::path::Path;

use deepcore_shared::content::ContentRegistry;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data")
}

#[test]
fn content_loads_and_validates() {
    let reg = ContentRegistry::load_dir(data_dir()).expect("content must load and validate");
    assert!(reg.blocks().len() >= 90, "blocks: {}", reg.blocks().len());
    assert!(reg.items().len() >= 180, "items: {}", reg.items().len());
    assert!(reg.recipes().len() >= 100, "recipes: {}", reg.recipes().len());
    assert!(reg.machines().len() >= 38, "machines: {}", reg.machines().len());
    assert!(reg.tech_nodes().len() >= 25, "tech: {}", reg.tech_nodes().len());
    assert_eq!(reg.layers().len(), 6);
}

#[test]
fn required_blocks_and_items_exist() {
    let reg = ContentRegistry::load_dir(data_dir()).unwrap();
    for id in [
        "air", "bedrock", "grass", "dirt", "stone", "granite", "basalt", "gneiss",
        "mantle_rock", "coal_seam", "malachite", "hematite", "galena", "pitchblende",
        "dense_pitchblende", "water",
    ] {
        assert!(reg.block(id).is_some(), "missing block {id}");
    }
    for id in [
        "steel_pickaxe", "hardened_steel_pickaxe", "steam_drill",
        "tungsten_carbide_drill", "nuclear_drill", "steel_ingot", "raw_iron",
        "yellowcake", "compact_core_cell",
    ] {
        assert!(reg.item(id).is_some(), "missing item {id}");
    }
}

#[test]
fn every_era_has_tech_nodes() {
    let reg = ContentRegistry::load_dir(data_dir()).unwrap();
    for era in 0..=5u8 {
        assert!(
            reg.tech_nodes().iter().any(|t| t.era == era),
            "no tech node in era {era}"
        );
    }
}

#[test]
fn content_hash_is_stable() {
    let a = ContentRegistry::load_dir(data_dir()).unwrap();
    let b = ContentRegistry::load_dir(data_dir()).unwrap();
    assert_eq!(a.hash, b.hash);
}

#[test]
fn tools_have_increasing_tiers() {
    let reg = ContentRegistry::load_dir(data_dir()).unwrap();
    use deepcore_shared::content::ItemCategory;
    let tier = |id: &str| {
        let item = reg.item(id).unwrap();
        match &reg.item_def(item).unwrap().category {
            ItemCategory::Tool(t) => t.tier,
            _ => panic!("{id} is not a tool"),
        }
    };
    assert!(tier("steel_pickaxe") < tier("hardened_steel_pickaxe"));
    assert!(tier("hardened_steel_pickaxe") < tier("steam_drill"));
    assert!(tier("steam_drill") < tier("tungsten_carbide_drill"));
    assert!(tier("tungsten_carbide_drill") < tier("nuclear_drill"));
}

#[test]
fn nuclear_drill_mines_a_three_by_three_face() {
    let reg = ContentRegistry::load_dir(data_dir()).unwrap();
    use deepcore_shared::content::ItemCategory;
    let item = reg.item("nuclear_drill").unwrap();
    match &reg.item_def(item).unwrap().category {
        ItemCategory::Tool(t) => {
            assert_eq!(t.area, (3, 3));
            assert!(t.speed >= 12.0);
        }
        other => panic!("unexpected category {other:?}"),
    }
}

#[test]
fn nuclear_chain_recipes_exist() {
    let reg = ContentRegistry::load_dir(data_dir()).unwrap();
    for id in [
        "crusher_uranium", "leach_uranium", "precipitate_yellowcake", "convert_uf6",
        "centrifuge_enrich", "fabricate_fuel_rod", "fabricate_core_cell",
        "shop_nuclear_drill", "pool_spent_fuel", "shop_control_rod",
    ] {
        assert!(reg.recipe(id).is_some(), "missing nuclear recipe {id}");
    }
}
