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
    stock: Option<(economy_rules::Account, u8, u8, [u8; 4])>,
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
        local originalPaperDollClick = PaperDollItemSlotButton_OnClick
        function PaperDollItemSlotButton_OnClick(button)
            if button == 'LeftButton' and this then
                local id=this:GetID()
                if id==16 or id==5 or id==11 or id==8 then __survivors_gear_toggle=id; return end
            end
            -- Arbitrary item movement is not represented by the portable inventory.
        end
        local originalBagClick = ContainerFrameItemButton_OnClick
        local originalMerchantShow = MerchantFrame_OnShow
        function MerchantFrame_OnShow()
            if originalMerchantShow then originalMerchantShow() end
            -- Survivors opens bags only when the player requests them.
            CloseAllBags()
        end
        function ContainerFrameItemButton_OnClick(button)
            if button=='RightButton' and this then
                local id=this:GetID()
                local bag=this:GetParent():GetID(); local items=__survivors_bag_ids and __survivors_bag_ids[bag]
                if items and items[id] then __survivors_item_equip=items[id]; return end
            end
            if originalBagClick then return originalBagClick(button) end
        end
    "#,
    ) {
        state.errors.push(e.to_string());
    }
    script.set_unit(
        "NPC",
        Some(UnitState {
            exists: true,
            name: Some("Marchand d'équipement".into()),
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
    let mut merchant = MerchantState {
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
    };
    for slot in 0..4 {
        merchant.items.push(MerchantItem {
            name: Some(format!(
                "{} commune",
                ["Arme", "Armure", "Bague", "Paire de bottes"][slot]
            )),
            texture: Some(gear_icon(8, slot)),
            price: 600_000,
            quantity: 1,
            num_available: -1,
            item_id: 700_000 + slot as u32,
            max_stack: Some(1),
            link: Some(format!(
                "|cffffffff|Hitem:{}|h[{}]|h|r",
                700_000 + slot as u32,
                [
                    "Arme commune",
                    "Armure commune",
                    "Bague commune",
                    "Bottes communes"
                ][slot]
            )),
            ..Default::default()
        });
    }
    merchant
}

fn gear_icon(class: u8, slot: usize) -> String {
    let icon = match slot {
        0 => match class {
            1 | 2 => "INV_Sword_04",
            3 => "INV_Weapon_Bow_07",
            4 => "INV_Weapon_ShortBlade_05",
            7 => "INV_Mace_01",
            _ => "INV_Staff_13",
        },
        1 => match class {
            1 | 2 => "INV_Chest_Plate03",
            3 | 7 => "INV_Chest_Chain",
            4 | 11 => "INV_Chest_Leather_01",
            _ => "INV_Chest_Cloth_17",
        },
        2 => "INV_Jewelry_Ring_03",
        _ => "INV_Boots_05",
    };
    format!(r"Interface\Icons\{icon}")
}

fn feed(
    script: Option<NonSendMut<UiScript>>,
    economy: Res<Economy>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    selection: Res<Selection>,
    mut booth: ResMut<crate::portrait::PaperDollBooth>,
    mut state: ResMut<StockUi>,
    progress: Res<super::character_save::TalentProgressStore>,
) {
    let Some(mut script) = script else { return };
    if !state.ready {
        return;
    }
    let available =
        *phase.get() == Phase::Preparing || (*phase.get() != Phase::Menu && run.dead_t.is_none());
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
    let class = if matches!(*phase.get(), Phase::Menu | Phase::Preparing) {
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
            level: if *phase.get() == Phase::Preparing {
                progress.0.highest_level(class.id)
            } else {
                run.level
            },
            health: run.hp.max(0.0) as u32,
            max_health: run.stats().max_hp as u32,
            ..Default::default()
        }),
    );
    let stock = (
        economy.account.clone(),
        economy.stock,
        class.id,
        run.transient_gear,
    );
    if state.stock.as_ref() != Some(&stock) {
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
        for slot in 0..4 {
            script.set_item_template(
                700_000 + slot as u32,
                ItemTemplateView {
                    name: [
                        "Arme commune",
                        "Armure commune",
                        "Bague commune",
                        "Bottes communes",
                    ][slot]
                        .into(),
                    quality: 1,
                    inventory_type: [13, 5, 11, 8][slot],
                    class: if slot == 0 { 2 } else { 4 },
                    icon: Some(gear_icon(class.id, slot)),
                    stackable: 1,
                    spell_triggers: vec![(1, 0, economy_rules::gear_bonus(slot, 1))],
                    description:
                        "Équipement permanent du camp. Peut être équipé ou fusionné chez l’artisan."
                            .into(),
                    ..Default::default()
                },
            );
        }
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
        let mut equipped = std::array::from_fn(|_| None);
        let mut bags = vec![bag];
        bags.extend((1..=4).map(|i| ContainerState {
            name: Some(format!("Sac de butin {i}")),
            num_slots: 16,
            ..Default::default()
        }));
        let mut bag_index = 1usize; // Potion reserves the first backpack slot.
        let mut mapping = String::from("__survivors_bag_ids={[0]={},[1]={},[2]={},[3]={},[4]={}};");
        for item in &economy.account.inventory {
            let slot = item.slot;
            let tier = item.tier;
            let live_slot = [16, 5, 11, 8][slot];
            let item_id = economy_rules::item_template_id(slot, item.quality, tier).unwrap();
            let quality = u32::from(item.quality) + 1;
            let color =
                ["ffffffff", "ff1eff00", "ff0070dd", "ffa335ee", "ffff8000"][item.quality as usize];
            let name = format!(
                "{} — {} · rang {tier}",
                economy_rules::gear_name(class.id, slot),
                ["Commun", "Inhabituel", "Rare", "Épique", "Légendaire"][item.quality as usize]
            );
            let icon = gear_icon(class.id, slot);
            let link = format!("|c{color}|Hitem:{item_id}|h[{name}]|h|r");
            script.set_item_template(item_id, ItemTemplateView {
                name: name.clone(), quality, inventory_type: [13,5,11,8][slot],
                class: if slot==0 {2} else {4}, icon: Some(icon.clone()), stackable: 1,
                spell_triggers: vec![(1,0,economy_rules::gear_bonus(slot,tier))],
            description: "Butin permanent. Fusion : trois copies blanches identiques non équipées donnent une copie verte.".into(),
                ..Default::default()
            });
            if economy.account.worn[slot] == Some(item.id) && economy.account.equipped[slot] {
                equipped[live_slot] = Some(InvSlotView {
                    item_id,
                    icon: Some(icon),
                    count: 1,
                    quality: quality as i32,
                    name: Some(name),
                    link: Some(link),
                    locked: true,
                    equip_slots: vec![live_slot as u8],
                    ..Default::default()
                });
            } else {
                let bag_id = bag_index / 16;
                let position = bag_index % 16 + 1;
                bags[bag_id].slots.insert(
                    position as u32,
                    ContainerSlot {
                        texture: Some(icon),
                        count: 1,
                        item_id,
                        quality: Some(quality),
                        link: Some(link),
                        ..Default::default()
                    },
                );
                mapping.push_str(&format!(
                    "__survivors_bag_ids[{bag_id}][{position}]={};",
                    item.id
                ));
                bag_index += 1;
            }
        }
        for bag_id in 1..=4 {
            equipped[19 + bag_id] = Some(InvSlotView {
                item_id: 800_000 + bag_id as u32,
                icon: Some(r"Interface\Icons\INV_Misc_Bag_08".into()),
                count: 1,
                quality: 1,
                name: Some(format!("Sac de butin {bag_id}")),
                ..Default::default()
            });
        }
        let _ = script.run(&mapping);
        for (id, bag) in bags.into_iter().enumerate() {
            script.set_container(id as i64, Some(bag));
            script.fire_event("BAG_UPDATE", vec![ScriptValue::Int(id as i64)]);
        }
        let summary = format!(
            "{} objets permanents · {} or. Les pièces portées sont protégées. {}",
            economy.account.inventory.len(),
            economy.account.gold,
            state.notice
        );
        let _ = script.run(&format!("__survivors_fusion_summary={:?}", summary));
        let lua = script.lua();
        if let Ok(rows) = lua.create_table() {
            for slot in 0..4 {
                let Ok(row) = lua.create_table() else {
                    continue;
                };
                let ids = economy.account.fusion_candidates(slot);
                let _ = row.set("count", ids.len());
                if let Some(first_id) = ids.first() {
                    let item = economy
                        .account
                        .inventory
                        .iter()
                        .find(|i| i.id == *first_id)
                        .unwrap();
                    let icon = gear_icon(class.id, slot);
                    let name = economy_rules::gear_name(class.id, slot);
                    let tooltip = format!(
                        "{name} — rang {}\n{}\nRecette : trois copies blanches identiques non équipées.",
                        item.tier,
                        economy_rules::gear_bonus(slot, item.tier)
                    );
                    if let Ok(ingredients) = lua.create_table() {
                        for index in 1..=ids.len() {
                            if let Ok(ingredient) = lua.create_table() {
                                let _ = ingredient.set("icon", icon.clone());
                                let _ = ingredient.set("count", 1);
                                let _ = ingredient.set("tooltip", tooltip.clone());
                                let _ = ingredients.set(index, ingredient);
                            }
                        }
                        let _ = row.set("ingredients", ingredients);
                    }
                    let _ = row.set("result_icon", icon);
                    let result_tier = economy_rules::fusion_result_tier(item.tier)
                        .unwrap_or(economy_rules::MAX_TIER);
                    let _ = row.set(
                        "result_name",
                        format!("{name} — qualité verte · rang {result_tier}"),
                    );
                    let _ = row.set("result_tier", result_tier);
                    let _ = row.set(
                        "result_tooltip",
                        format!(
                            "{name} — qualité verte · rang {result_tier}\n{}\nObjet permanent.",
                            economy_rules::gear_bonus(slot, result_tier)
                        ),
                    );
                }
                let _ = rows.set(slot + 1, row);
            }
            let _ = lua.globals().set("__survivors_fusion_slots", rows);
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
    if script
        .eval::<bool>("return __survivors_merchant or false")
        .unwrap_or(false)
    {
        let _ = script.run("__survivors_merchant=nil");
        state.merchant_requested = true;
    }
    if let Ok(Some(id)) = script.eval::<Option<u64>>("return __survivors_item_equip") {
        let _ = script.run("__survivors_item_equip=nil");
        match economy.account.equip_item(id) {
            Ok(_) => {
                super::economy::sync_gear(&mut run, &economy.account);
                run.hp = run.hp.min(run.stats().max_hp);
                run.version = run.version.wrapping_add(1);
            }
            Err(message) => {
                script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(message.into())])
            }
        }
    }
    if let Ok(Some(slot)) = script.eval::<Option<usize>>("return __survivors_fuse_slot") {
        let _ = script.run("__survivors_fuse_slot=nil");
        if *phase.get() == Phase::Preparing {
            let ids = economy.account.fusion_ingredients(slot);
            let result = ids
                .ok_or("Il faut 3 objets non équipés de même qualité et emplacement.")
                .and_then(|ids| economy.account.fuse_items(ids));
            let message = match result {
                Ok(_) => "Fusion réussie : objet amélioré dans vos sacs.",
                Err(message) => message,
            };
            state.notice = message.into();
            let _ = script.run(&format!("__survivors_fusion_summary={:?}", message));
            script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(message.into())]);
        }
    }
    if let Ok(Some(native_slot)) = script.eval::<Option<usize>>("return __survivors_gear_toggle") {
        let _ = script.run("__survivors_gear_toggle=nil");
        if let Some(slot) = [16, 5, 11, 8].iter().position(|id| *id == native_slot) {
            if let Err(message) = super::economy::toggle_gear(&mut economy, &mut run, slot) {
                script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(message.into())]);
            }
        }
    }
    if let Ok(Some(slot)) = script.eval::<Option<usize>>("return __survivors_gear_equip") {
        let _ = script.run("__survivors_gear_equip=nil");
        if let Err(message) = super::economy::equip_gear(&mut economy, &mut run, slot, true) {
            script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(message.into())]);
        }
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
        if !state.merchant_shown || quantity == 0 {
            continue;
        }
        if (2..=5).contains(&row) {
            if *phase.get() != Phase::Preparing {
                continue;
            }
            let mut account = economy.account.clone();
            let result = if quantity == 1 && account.gold >= 60 {
                account.grant_boss_item(row as usize - 2, 0, 1).map(|_| {
                    account.gold -= 60;
                })
            } else {
                Err("Cet objet coûte 60 or. Achetez une pièce à la fois.")
            };
            match result {
                Ok(()) => {
                    economy.account = account;
                    state.notice = "Équipement acheté : dans vos sacs.".into();
                }
                Err(message) => {
                    script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(message.into())])
                }
            }
            continue;
        }
        if row != 1 {
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
    // Inventory, character and talents remain nonmodal, as in the playable WoW world.
    state.modal_open = script.eval::<bool>("return (GameMenuFrame and GameMenuFrame:IsShown()) or (OptionsFrame and OptionsFrame:IsShown()) or (SoundOptionsFrame and SoundOptionsFrame:IsShown()) or (UIOptionsFrame and UIOptionsFrame:IsShown()) or (KeyBindingFrame and KeyBindingFrame:IsShown()) or (MerchantFrame and MerchantFrame:IsShown()) or (StackSplitFrame and StackSplitFrame:IsShown()) or false").unwrap_or(false);
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
                name: Some("Marchand d'équipement".into()),
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
