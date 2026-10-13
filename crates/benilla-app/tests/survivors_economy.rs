#[path = "../src/survivors/economy_rules.rs"]
#[allow(dead_code)]
mod rules;

#[test]
fn purchases_are_atomic_and_stock_limited_even_after_drinking() {
    let mut a = rules::Account {
        gold: 100,
        potions: 0,
        gear: [0; 4],
    };
    let mut stock = 3;
    for _ in 0..3 {
        a.buy_potion(&mut stock).unwrap();
    }
    assert_eq!((a.gold, a.potions, stock), (40, 3, 0));
    let before = a.clone();
    assert!(a.buy_potion(&mut stock).is_err());
    assert_eq!(a, before);
    a.drink(10.0, 100.0, &mut 0.0).unwrap();
    assert!(a.buy_potion(&mut stock).is_err());
}
#[test]
fn insufficient_funds_never_take_stock_or_gear() {
    let mut a = rules::Account::default();
    let mut stock = 3;
    let mut opened = 0;
    assert!(a.buy_potion(&mut stock).is_err());
    assert!(a.open_chest(&mut opened, 0).is_err());
    assert_eq!((a, stock, opened), (rules::Account::default(), 3, 0));
}
#[test]
fn chests_upgrade_available_slots_and_charge_increasing_price() {
    let mut a = rules::Account {
        gold: 1000,
        potions: 0,
        gear: [8, 0, 0, 0],
    };
    let mut opened = 0;
    assert_eq!(a.open_chest(&mut opened, 0).unwrap(), 1);
    assert_eq!((a.gear, a.gold, opened), ([8, 1, 0, 0], 960, 1));
    a.open_chest(&mut opened, 0).unwrap();
    assert_eq!(a.gold, 900);
    a.gear = [8; 4];
    let before = a.clone();
    assert!(a.open_chest(&mut opened, 0).is_err());
    assert_eq!(a, before);
}
#[test]
fn potion_respects_max_hp_shared_cooldown_and_death() {
    let mut a = rules::Account {
        potions: 3,
        ..Default::default()
    };
    let mut cd = 0.0;
    assert_eq!(a.drink(70.0, 100.0, &mut cd).unwrap(), 100.0);
    assert_eq!(cd, 20.0);
    assert!(a.drink(10.0, 100.0, &mut cd).is_err());
    assert_eq!(a.potions, 2);
    cd = 0.0;
    assert!(a.drink(100.0, 100.0, &mut cd).is_err());
    assert!(a.drink(0.0, 100.0, &mut cd).is_err());
    assert_eq!(a.potions, 2);
}
#[test]
fn classes_receive_appropriate_weapon_and_armour_names() {
    assert_eq!(rules::gear_name(9, 0), "Bâton");
    assert_eq!(rules::gear_name(9, 1), "Robe de tissu");
    assert_eq!(rules::gear_name(2, 0), "Épée");
    assert_eq!(rules::gear_name(2, 1), "Armure de plaques");
    assert_eq!(rules::gear_name(3, 0), "Arc");
    assert_eq!(rules::gear_name(4, 0), "Dagues");
}
