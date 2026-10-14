//! Offline providers for a deliberately small set of genuine Blizzard 1.12 FrameXML windows.
//! This does not boot the normal network UI, the modern benilla layer, or player addons.
use super::{economy::Economy, economy_rules, Phase, Run, Selection};
use benilla_assets::{LockRecover, WorldAssets};
use benilla_ui::script::{
    ContainerSlot, ContainerState, InvSlotView, ItemTemplateView, MerchantItem, MerchantState,
    ScriptValue, UiScript, UnitState,
};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct StockUi {
    pub ready: bool,
    pub merchant_requested: bool,
    pub menu_requested: bool,
    pub return_requested: bool,
    pub modal_open: bool,
    pub notice: String,
    pub errors: Vec<String>,
    session: Option<u64>,
    merchant_shown: bool,
    menu_shown: bool,
    stock: Option<(u32, u8, u8, [u8; 4], u8)>,
    bindings_seeded: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<StockUi>()
        .add_systems(Update, load.before(crate::ui_script::UiFeed))
        .add_systems(
            Update,
            seed_bindings.after(load).before(crate::ui_script::UiFeed),
        )
        .add_systems(Update, feed.in_set(crate::ui_script::UiFeed))
        .add_systems(Update, drain.after(crate::ui_script::UiInput))
        .add_systems(OnEnter(Phase::Menu), close_windows);
}

fn close_windows(script: Option<NonSendMut<UiScript>>, mut state: ResMut<StockUi>) {
    if state.ready {
        if let Some(mut script) = script {
            if let Err(e) = script.run("CloseAllWindows()") {
                state.errors.push(e.to_string());
            }
        }
    }
    state.merchant_requested = false;
    state.menu_requested = false;
    state.merchant_shown = false;
    state.menu_shown = false;
    state.modal_open = false;
}

#[path = "blizzard_frames.rs"]
mod frames;
#[path = "talent_tooltips.rs"]
pub(super) mod talent_tooltips;

fn load(
    assets: Option<Res<WorldAssets>>,
    script: Option<NonSendMut<UiScript>>,
    mut state: ResMut<StockUi>,
) {
    let (Some(assets), Some(mut script)) = (assets, script) else {
        return;
    };
    let session = script.session();
    if state.session == Some(session) {
        return;
    }
    state.session = Some(session);
    state.ready = false;
    state.bindings_seeded = false;
    state.merchant_shown = false;
    state.menu_shown = false;
    state.errors.clear();
    script.register_cvars(crate::cvars::registered_pairs());
    let read = |path: &str| assets.chain.lock_recover().read_file(path).ok();
    state.errors = frames::load_windows(&script, &read);
    if let Err(e) = script.run(
        r#"
        if GameMenuButtonLogout then
            GameMenuButtonLogout:SetText("Sélection du personnage")
            GameMenuButtonLogout:SetScript("OnClick", function()
                __survivors_return = true
                HideUIPanel(GameMenuFrame)
            end)
        end
        -- These panels are server services, unavailable in the local adventure.
        if GameMenuButtonHelp then GameMenuButtonHelp:Disable() end
        if GameMenuButtonMacros then GameMenuButtonMacros:Disable() end
        -- The adventure preserves talent progress between runs, separately from run level.
        __survivors_talent_level = __survivors_talent_level or 1
        __survivors_talents_available = false
        function ToggleTalentFrame()
            if not __survivors_talents_available then return end
            if (__survivors_talent_level or 1) < 10 then
                if UIErrorsFrame then UIErrorsFrame:AddMessage("Les talents se débloquent au niveau 10 de cette classe.", 1, 0.2, 0.2) end
                return
            end
            TalentFrame_Toggle()
        end
        if TalentMicroButton then
            TalentMicroButton:SetParent(UIParent)
            TalentMicroButton:ClearAllPoints()
            TalentMicroButton:SetPoint("BOTTOMRIGHT", UIParent, "BOTTOMRIGHT", -8, 54)
            TalentMicroButton:Show()
        end
    "#,
    ) {
        state.errors.push(e.to_string());
    }
    script.set_unit(
        "NPC",
        Some(UnitState {
            exists: true,
            name: Some("Marchand de potions".into()),
            ..Default::default()
        }),
    );
    script.fire_event("VARIABLES_LOADED", vec![]);
    state.errors.extend(script.errors());
    state.ready = state.errors.is_empty();
    state.stock = None;
    if state.ready {
        info!("Survivors: fenêtres Blizzard hors ligne chargées");
    } else {
        error!(
            "Survivors: chargement Blizzard incomplet: {:?}",
            state.errors
        );
    }
}

fn seed_bindings(world: &mut World) {
    let state = world.resource::<StockUi>();
    if !state.ready || state.bindings_seeded {
        return;
    }
    let Some(mut script) = world.remove_non_send_resource::<UiScript>() else {
        return;
    };
    crate::bindings::seed_bindings_for_vm(world, &mut script);
    world.insert_non_send_resource(script);
    world.resource_mut::<StockUi>().bindings_seeded = true;
}

fn merchant(economy: &Economy) -> MerchantState {
    MerchantState {
        items: vec![MerchantItem {
            name: Some("Potion de soins".into()),
            texture: Some(r"Interface\Icons\INV_Potion_51".into()),
            price: economy_rules::POTION_PRICE.saturating_mul(10_000),
            quantity: 1,
            num_available: i32::from(economy.stock),
            item_id: 118,
            max_stack: Some(u32::from(economy_rules::POTION_LIMIT)),
            link: Some("|cffffffff|Hitem:118|h[Potion de soins]|h|r".into()),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn feed(
    script: Option<NonSendMut<UiScript>>,
    economy: Res<Economy>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    selection: Res<Selection>,
    mut booth: ResMut<crate::portrait::PaperDollBooth>,
    mut state: ResMut<StockUi>,
) {
    let Some(mut script) = script else { return };
    if !state.ready {
        return;
    }
    let available = *phase.get() != Phase::Menu && run.dead_t.is_none();
    let _ = script
        .lua()
        .globals()
        .set("__survivors_talents_available", available);
    let micro = if available {
        "if TalentMicroButton then TalentMicroButton:Enable(); TalentMicroButton:Show() end"
    } else {
        "if TalentMicroButton then TalentMicroButton:Disable(); TalentMicroButton:Hide() end"
    };
    if let Err(error) = script.run(micro) {
        state.errors.push(error.to_string());
    }
    booth.yaw = script.model_pane_facing("CharacterModelFrame");
    let class = if *phase.get() == Phase::Menu {
        selection.class_def().unwrap_or(run.class)
    } else {
        run.class
    };
    let class_file = class.name.to_ascii_uppercase();
    let (race, sex) = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map(|&(_, race, sex)| (race, sex))
        .unwrap_or((run.race.max(1), 0));
    let class_name = script
        .eval::<Option<String>>(&format!(
            "return LOCALIZED_CLASS_NAMES_MALE and LOCALIZED_CLASS_NAMES_MALE['{class_file}']"
        ))
        .ok()
        .flatten()
        .unwrap_or_else(|| super::localization::class_name(class.id).into());
    script.set_unit(
        "player",
        Some(UnitState {
            exists: true,
            has_object: true,
            is_player: true,
            player_controlled: true,
            name: Some(selection.name.clone()),
            race: Some(super::localization::race_name(race).into()),
            race_file: crate::ui_unit::race_names(race).map(|(_, file)| file.into()),
            sex: sex + 2,
            class: Some(class_name),
            class_file: Some(class_file),
            level: run.level,
            health: run.hp.max(0.0) as u32,
            max_health: run.stats().max_hp as u32,
            ..Default::default()
        }),
    );
    let stock = (
        economy.account.gold,
        economy.account.potions,
        economy.stock,
        economy.account.gear,
        class.id,
    );
    if state.stock != Some(stock) {
        state.stock = Some(stock);
        script.set_item_template(
            118,
            ItemTemplateView {
                name: "Potion de soins".into(),
                quality: 1,
                class: 0,
                subclass: 1,
                stackable: u32::from(economy_rules::POTION_LIMIT),
                icon: Some(r"Interface\Icons\INV_Potion_51".into()),
                spell_triggers: vec![(
                    0,
                    0,
                    "Rend 50 % de votre maximum de points de vie. Recharge : 20 s.".into(),
                )],
                description: "Trois potions au maximum. Stock limité à trois achats par partie."
                    .into(),
                ..Default::default()
            },
        );
        script.set_money(u64::from(economy.account.gold).saturating_mul(10_000));
        script.set_merchant(Some(merchant(&economy)));
        let mut bag = ContainerState {
            name: Some("Sac à dos".into()),
            num_slots: 16,
            ..Default::default()
        };
        if economy.account.potions > 0 {
            bag.slots.insert(
                1,
                ContainerSlot {
                    texture: Some(r"Interface\Icons\INV_Potion_51".into()),
                    count: u32::from(economy.account.potions),
                    item_id: 118,
                    quality: Some(1),
                    bar_placeable: true,
                    link: Some("|cffffffff|Hitem:118|h[Potion de soins]|h|r".into()),
                    ..Default::default()
                },
            );
        }
        script.set_container(0, Some(bag));
        let mut equipped = std::array::from_fn(|_| None);
        for (slot, &tier) in economy
            .account
            .gear
            .iter()
            .enumerate()
            .filter(|(_, tier)| **tier > 0)
        {
            let live_slot = [16, 5, 11, 8][slot];
            let item_id = 900_001 + slot as u32;
            let name = format!("{} — rang {tier}", economy_rules::gear_name(class.id, slot));
            let icon = match slot {
                0 => match class.id {
                    1 | 2 => "INV_Sword_04",
                    3 => "INV_Weapon_Bow_07",
                    4 => "INV_Weapon_ShortBlade_05",
                    7 => "INV_Mace_01",
                    _ => "INV_Staff_13",
                },
                1 => match class.id {
                    1 | 2 => "INV_Chest_Plate03",
                    3 | 7 => "INV_Chest_Chain",
                    4 | 11 => "INV_Chest_Leather_01",
                    _ => "INV_Chest_Cloth_17",
                },
                2 => "INV_Jewelry_Ring_03",
                _ => "INV_Boots_05",
            };
            let icon = format!(r"Interface\Icons\{icon}");
            let link = format!("|cff1eff00|Hitem:{item_id}|h[{name}]|h|r");
            script.set_item_template(
                item_id,
                ItemTemplateView {
                    name: name.clone(),
                    quality: 2,
                    inventory_type: [13, 5, 11, 8][slot],
                    class: if slot == 0 { 2 } else { 4 },
                    icon: Some(icon.clone()),
                    stackable: 1,
                    spell_triggers: vec![(1, 0, economy_rules::gear_bonus(slot, tier))],
                    description:
                        "Équipement obtenu dans un coffre. Bonus actif dans cette aventure.".into(),
                    ..Default::default()
                },
            );
            equipped[live_slot] = Some(InvSlotView {
                item_id,
                icon: Some(icon),
                count: 1,
                quality: 2,
                name: Some(name),
                link: Some(link),
                locked: true,
                equip_slots: vec![live_slot as u8],
                ..Default::default()
            });
        }
        script.set_inventory_slots(equipped);
        script.fire_event(
            "UNIT_INVENTORY_CHANGED",
            vec![ScriptValue::Str("player".into())],
        );
        script.fire_event("PLAYER_MONEY", vec![]);
        script.fire_event("MERCHANT_UPDATE", vec![]);
    }
    if *phase.get() == Phase::Menu {
        state.merchant_requested = false;
    }
    if state.merchant_requested != state.merchant_shown {
        state.merchant_shown = state.merchant_requested;
        script.fire_event(
            if state.merchant_shown {
                "MERCHANT_SHOW"
            } else {
                "MERCHANT_CLOSED"
            },
            vec![],
        );
        let _ = script.take_merchant_close();
    }
    if state.menu_requested != state.menu_shown {
        state.menu_shown = state.menu_requested;
        let cmd = if state.menu_shown {
            "ShowUIPanel(GameMenuFrame)"
        } else {
            "HideUIPanel(GameMenuFrame)"
        };
        if let Err(e) = script.run(cmd) {
            state.errors.push(e.to_string());
        }
    }
}

fn drain(
    script: Option<NonSendMut<UiScript>>,
    phase: Res<State<Phase>>,
    mut economy: ResMut<Economy>,
    mut run: ResMut<Run>,
    mut state: ResMut<StockUi>,
) {
    let Some(mut script) = script else { return };
    if !state.ready {
        return;
    }
    for (bag, slot) in script.take_container_uses() {
        if bag == 0 && slot == 1 && matches!(*phase.get(), Phase::Playing | Phase::Paused) {
            let max_hp = run.stats().max_hp;
            let economy = &mut *economy;
            match economy
                .account
                .drink(run.hp, max_hp, &mut economy.potion_cd)
            {
                Ok(hp) => run.hp = hp,
                Err(e) => script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(e.into())]),
            }
        }
    }
    for (row, quantity) in script.take_merchant_buys() {
        if !state.merchant_shown || row != 1 || quantity == 0 {
            continue;
        }
        // Validate the entire requested stack first: a failing purchase never partly spends gold.
        let mut account = economy.account.clone();
        let mut stock = economy.stock;
        let mut result = Ok(());
        for _ in 0..quantity.min(4) {
            if let Err(e) = account.buy_potion(&mut stock) {
                result = Err(e);
                break;
            }
        }
        if quantity > u32::from(economy_rules::POTION_LIMIT) {
            result = Err("Vous portez au maximum 3 potions.");
        }
        match result {
            Ok(()) => {
                economy.account = account;
                economy.stock = stock;
                state.notice = "Potion achetée.".into();
            }
            Err(e) => {
                state.notice = e.into();
                script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(e.into())]);
            }
        }
    }
    if script.take_merchant_close() {
        state.merchant_requested = false;
        state.merchant_shown = false;
    }
    let menu_visible = script
        .eval::<bool>("return GameMenuFrame and GameMenuFrame:IsShown() or false")
        .unwrap_or(false);
    if menu_visible != state.menu_shown {
        state.menu_requested = menu_visible;
        state.menu_shown = menu_visible;
    }
    if script
        .eval::<bool>("return __survivors_return or false")
        .unwrap_or(false)
    {
        state.return_requested = true;
        let _ = script.run("__survivors_return = false");
    }
    state.modal_open = script.eval::<bool>("return (GameMenuFrame and GameMenuFrame:IsShown()) or (OptionsFrame and OptionsFrame:IsShown()) or (SoundOptionsFrame and SoundOptionsFrame:IsShown()) or (UIOptionsFrame and UIOptionsFrame:IsShown()) or (KeyBindingFrame and KeyBindingFrame:IsShown()) or (MerchantFrame and MerchantFrame:IsShown()) or (CharacterFrame and CharacterFrame:IsShown()) or (TalentFrame and TalentFrame:IsShown()) or (StackSplitFrame and StackSplitFrame:IsShown()) or (IsBagOpen and IsBagOpen(0) ~= nil) or false").unwrap_or(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genuine_offline_merchant_and_options_open_without_network_or_lua_errors() {
        let data = benilla_formats::wow_data_or_skip!();
        let chain = benilla_formats::open_chain(&data).unwrap();
        let mut script = UiScript::new().unwrap();
        script.set_screen_size(1024.0, 768.0);
        script.register_cvars(crate::cvars::registered_pairs());
        let errors = frames::load_windows(&script, &|path| chain.read(path).ok());
        assert!(errors.is_empty(), "{errors:#?}");
        script.set_unit(
            "player",
            Some(UnitState {
                exists: true,
                class: Some("Mage".into()),
                class_file: Some("MAGE".into()),
                ..Default::default()
            }),
        );
        script.set_unit(
            "NPC",
            Some(UnitState {
                exists: true,
                name: Some("Marchand de potions".into()),
                ..Default::default()
            }),
        );
        let economy = Economy::default();
        script.set_money(1_000_000);
        script.set_merchant(Some(merchant(&economy)));
        script.fire_event("VARIABLES_LOADED", vec![]);
        script.fire_event("MERCHANT_SHOW", vec![]);
        assert!(script
            .eval::<bool>("return MerchantFrame:IsShown()")
            .unwrap());
        script
            .run("MerchantItem1ItemButton:Click('RightButton')")
            .unwrap();
        assert_eq!(script.take_merchant_buys(), vec![(1, 1)]);
        for name in [
            "GameMenuFrame",
            "OptionsFrame",
            "SoundOptionsFrame",
            "UIOptionsFrame",
            "KeyBindingFrame",
        ] {
            script.run(&format!("ShowUIPanel({name})")).unwrap();
            assert!(
                script
                    .eval::<bool>(&format!("return {name}:IsShown()"))
                    .unwrap(),
                "{name}"
            );
        }
        assert!(script.errors().is_empty(), "{:?}", script.errors());
    }
}
