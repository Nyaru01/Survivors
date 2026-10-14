//! A preparation guide built from the mode's live tables and saved account state.
use super::{
    character_save::WorldProgressStore, data, economy::Economy, economy_rules, localization,
    meta_progress::MetaProgressStore, meta_rules, Phase, Run, Selection,
};
use benilla_ui::script::UiScript;
use bevy::prelude::*;

const PAGE: &str = r#"
if not SurvivorsGuideFrame then
    local f=CreateFrame('Frame','SurvivorsGuideFrame',UIParent)
    f:SetWidth(900); f:SetHeight(610); f:SetPoint('CENTER',UIParent,'CENTER',0,0)
    f:SetFrameStrata('DIALOG')
    f:SetBackdrop({bgFile='Interface\\DialogFrame\\UI-DialogBox-Background',edgeFile='Interface\\DialogFrame\\UI-DialogBox-Border',tile=true,tileSize=32,edgeSize=32,insets={left=11,right=12,top=12,bottom=11}})
    f:SetBackdropColor(0.04,0.03,0.02,0.98); f:SetBackdropBorderColor(0.78,0.62,0.28,1)
    local title=f:CreateFontString('SurvivorsGuideTitle','OVERLAY','GameFontNormalLarge')
    title:SetPoint('TOP',f,'TOP',0,-18); title:SetText('Guide de préparation')
    local tabs={'Personnage','Collection','Butin','Codex','Hauts faits','Récompenses'}
    for i=1,6 do
        local b=CreateFrame('Button','SurvivorsGuideTab'..i,f,'UIPanelButtonTemplate')
        b:SetWidth(132); b:SetHeight(28); b:SetPoint('TOPLEFT',f,'TOPLEFT',22+(i-1)*140,-55)
        b:SetText(tabs[i]); b:SetScript('OnClick',function() __survivors_guide_tab=i end)
    end
    local scroll=CreateFrame('ScrollFrame','SurvivorsGuideScroll',f,'UIPanelScrollFrameTemplate')
    scroll:SetPoint('TOPLEFT',f,'TOPLEFT',30,-98); scroll:SetPoint('BOTTOMRIGHT',f,'BOTTOMRIGHT',-46,66)
    local content=CreateFrame('Frame','SurvivorsGuideContent',scroll); content:SetWidth(820); content:SetHeight(800); scroll:SetScrollChild(content)
    local body=content:CreateFontString('SurvivorsGuideBody','OVERLAY','GameFontHighlight')
    body:SetPoint('TOPLEFT',content,'TOPLEFT',0,0); body:SetWidth(820); body:SetHeight(800)
    body:SetJustifyH('LEFT'); body:SetJustifyV('TOP')
    local claim=CreateFrame('Button','SurvivorsGuideClaim',f,'UIPanelButtonTemplate')
    claim:SetWidth(240); claim:SetHeight(30); claim:SetPoint('BOTTOMLEFT',f,'BOTTOMLEFT',24,20)
    claim:SetText('Réclamer les récompenses disponibles')
    claim:SetScript('OnClick',function() __survivors_guide_claim=true end)
    local close=CreateFrame('Button','SurvivorsGuideClose',f,'UIPanelButtonTemplate')
    close:SetWidth(120); close:SetHeight(30); close:SetPoint('BOTTOMRIGHT',f,'BOTTOMRIGHT',-24,20)
    close:SetText('Fermer'); close:SetScript('OnClick',function() __survivors_guide_close=true end)
    tinsert(UISpecialFrames,'SurvivorsGuideFrame')
    local open=CreateFrame('Button','SurvivorsGuideOpen',UIParent,'UIPanelButtonTemplate')
    open:SetWidth(150); open:SetHeight(30); open:SetPoint('BOTTOMRIGHT',UIParent,'BOTTOMRIGHT',-24,24)
    open:SetText('Guide de préparation')
    open:SetScript('OnClick',function() f:Show(); __survivors_guide_open=true end)
    f:Hide(); open:Hide()
end
function SurvivorsGuideSetContent(heading,body,canClaim,showRewards)
    SurvivorsGuideTitle:SetText(heading)
    SurvivorsGuideBody:SetText(body)
    local _,lines=string.gsub(body,'\n','\n'); local h=math.max(500,lines*20+100); SurvivorsGuideContent:SetHeight(h); SurvivorsGuideBody:SetHeight(h)
    SurvivorsGuideClaim:SetAlpha(canClaim and 1 or 0.45)
    SurvivorsGuideClaim:EnableMouse(showRewards and canClaim)
    SurvivorsGuideClaim:SetText(canClaim and 'Réclamer les récompenses disponibles' or 'Aucune récompense disponible')
    if showRewards then SurvivorsGuideClaim:Show() else SurvivorsGuideClaim:Hide() end
end
function SurvivorsGuideSetVisible(preparing)
    if not preparing then SurvivorsGuideFrame:Hide() end
    if preparing then SurvivorsGuideOpen:Show() else SurvivorsGuideOpen:Hide() end
end
"#;

#[derive(Resource, Default)]
struct GuideState {
    session: u64,
    tab: usize,
    was_preparing: bool,
    refresh: f32,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<GuideState>()
        .add_systems(Update, guide.after(crate::ui_script::UiInput));
}

#[allow(clippy::too_many_arguments)]
fn guide(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    stock: Res<super::blizzard_ui::StockUi>,
    script: Option<NonSendMut<UiScript>>,
    selection: Res<Selection>,
    run: Res<Run>,
    worlds: Res<WorldProgressStore>,
    mut economy: ResMut<Economy>,
    mut meta: ResMut<MetaProgressStore>,
    mut state: ResMut<GuideState>,
) {
    let Some(mut script) = script else { return };
    if !stock.ready {
        return;
    }
    if state.session != script.session() {
        if let Err(error) = script.run(PAGE) {
            warn!("Guide survivants: {error}");
            return;
        }
        state.session = script.session();
        state.tab = 0;
        state.was_preparing = false;
    }
    let preparing = *phase.get() == Phase::Preparing;
    let _ = script.run(if preparing {
        "SurvivorsGuideSetVisible(true)"
    } else {
        "SurvivorsGuideSetVisible(false)"
    });
    if !preparing {
        state.was_preparing = false;
        return;
    }
    if !state.was_preparing {
        // Keep the player's stock panel stack clean when the preparation guide opens.
        let _ = script.run("CloseAllWindows()");
        state.was_preparing = true;
    }
    if keys.just_pressed(KeyCode::F11) {
        let _ = script.run("if SurvivorsGuideFrame:IsShown() then SurvivorsGuideFrame:Hide() else SurvivorsGuideFrame:Show() end");
    }
    if script
        .eval::<bool>("return __survivors_guide_close or false")
        .unwrap_or(false)
    {
        let _ = script.run("__survivors_guide_close=false; SurvivorsGuideFrame:Hide()");
    }
    if let Ok(Some(tab)) = script.eval::<Option<usize>>("return __survivors_guide_tab") {
        if (1..=6).contains(&tab) {
            state.tab = tab - 1;
        }
    }
    let mut claimed = false;
    if script
        .eval::<bool>("return __survivors_guide_claim or false")
        .unwrap_or(false)
    {
        let _ = script.run("__survivors_guide_claim=false");
        for achievement in meta_rules::ACHIEVEMENTS {
            if let Some(gold) = meta.0.claim(achievement.id) {
                economy.account.gold = economy.account.gold.saturating_add(gold);
                claimed = true;
            }
        }
    }
    state.refresh += time.delta_secs();
    if !claimed && state.refresh < 0.4 {
        return;
    }
    state.refresh = 0.0;
    let (title, body, can_claim) = content(state.tab, &selection, &run, &economy, &meta, &worlds);
    let _ = script.lua().globals().set("__survivors_guide_body", body);
    let _ = script.lua().globals().set("__survivors_guide_title", title);
    let _ = script
        .lua()
        .globals()
        .set("__survivors_guide_claimable", can_claim);
    let _ = script
        .lua()
        .globals()
        .set("__survivors_guide_show_rewards", state.tab == 5);
    let _ = script.run("SurvivorsGuideSetContent(__survivors_guide_title,__survivors_guide_body,__survivors_guide_claimable,__survivors_guide_show_rewards)");
}

fn content(
    tab: usize,
    selection: &Selection,
    run: &Run,
    economy: &Economy,
    meta: &MetaProgressStore,
    worlds: &WorldProgressStore,
) -> (String, String, bool) {
    let class = selection.class_def().unwrap_or(run.class);
    let race = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map_or(run.race, |look| look.1);
    let map_index = selection.map.min(data::MAPS.len().saturating_sub(1));
    let map = &data::MAPS[map_index];
    let roster = data::roster(map_index);
    let mut claimable = false;
    let body = match tab {
        0 => {
            let preview = Run {
                class,
                race,
                gear: economy.account.gear_effective(),
                ..default()
            };
            let stats = preview.stats();
            let (hp, might, speed, armor, regen, crit) = (
                stats.max_hp,
                stats.might,
                stats.speed,
                stats.armor,
                stats.regen,
                stats.crit,
            );
            let level = if run.class.id == class.id {
                run.level.max(1)
            } else {
                1
            };
            let mut out = format!("Personnage — {}\n{} · Niveau {} · {}\nZone choisie : {} ({})\n\nATTRIBUTS\nVie : {:.0}    Puissance : {:.2}\nVitesse : {:.1}    Armure : {:.0}\nRégénération : {:.1}    Critique : {:.0} %\n\nRACIAL\n{}\n\nSORTS DE CLASSE", selection.name, localization::race_name(race), level, localization::class_name(class.id), localization::zone_name(map.name).unwrap_or(map.name), map.zone, hp, might, speed, armor, regen, crit * 100.0, super::racials::description(race));
            for key in class.pool {
                let ability = data::ability(key);
                let name = localization::ability_name(key).unwrap_or(key);
                out.push_str(&format!(
                    "\n- {name} — {} · {:.0} dégâts · recharge {:.1} s",
                    kind_name(ability.kind),
                    ability.damage,
                    ability.cooldown
                ));
            }
            out
        }
        1 => {
            let slots = ["Arme", "Armure", "Anneau", "Bottes"];
            let mut out = format!("Collection permanente\nOr : {}    Potions : {}\nPièces conservées : {} / {}\n\nÉQUIPEMENT ACTUEL", economy.account.gold, economy.account.potions, economy.account.inventory.len(), economy_rules::MAX_INVENTORY);
            for (slot, label) in slots.iter().enumerate() {
                let gear = economy.account.gear_effective()[slot];
                out.push_str(&format!(
                    "\n{label} — rang {gear} ({})",
                    economy_rules::gear_bonus(slot, gear)
                ));
            }
            out.push_str("\n\nOBJETS DE BOSS");
            if economy.account.inventory.is_empty() {
                out.push_str("\nAucune pièce permanente pour le moment.");
            }
            for item in &economy.account.inventory {
                out.push_str(&format!(
                    "\n{} · qualité {} · rang {}",
                    economy_rules::gear_name(class.id, item.slot),
                    item.quality,
                    item.tier
                ));
            }
            out
        }
        2 => {
            let mut out = format!("Butin de {}\nLes boss apparaissent toutes les {} minutes. Leurs objets rejoignent la collection permanente.\n\nEMPLACEMENTS", localization::zone_name(map.name).unwrap_or(map.name), (data::BOSS_EVERY_SECS / 60.0) as u32);
            for slot in 0..4 {
                out.push_str(&format!(
                    "\n{} — {}",
                    ["Arme", "Armure", "Anneau", "Bottes"][slot],
                    economy_rules::gear_name(class.id, slot)
                ));
            }
            out.push_str("\n\nBOSS DE LA ZONE");
            for (i, boss) in roster.bosses.iter().enumerate() {
                out.push_str(&format!(
                    "\n{} — niveau {} · {} PV · {} dégâts",
                    boss.name, boss.level, boss.hp as u32, boss.damage as u32
                ));
                if i >= 7 {
                    break;
                }
            }
            out
        }
        3 => {
            let completed = worlds.0.profile(race).map_or(0, |p| p.completed.len());
            let number = worlds.0.world_number(race, map_index).map_or(1, |n| n);
            let mut out = format!("Codex — {}\n{} · Monde {} · {} boss requis pour valider une zone.\nLes boss arrivent toutes les 3 minutes; quatre victoires ouvrent la suite de votre route.\nZones validées : {}\n\nBOSS", localization::zone_name(map.name).unwrap_or(map.name), map.zone, number, super::progression_world::BOSSES_REQUIRED, completed);
            for boss in &roster.bosses {
                out.push_str(&format!(
                    "\n{} — niveau {} · {:.0} PV · {:.0} dégâts{}",
                    boss.name,
                    boss.level,
                    boss.hp,
                    boss.damage,
                    if boss.big { " · grand" } else { "" }
                ));
            }
            out.push_str(&format!(
                "\n\nENNEMIS RÉPERTORIÉS : {}",
                roster.enemies.len()
            ));
            out
        }
        4 | 5 => {
            let mut out = format!(
                "{}\nProgression cumulée conservée entre les expéditions.",
                if tab == 4 {
                    "Hauts faits"
                } else {
                    "Récompenses"
                }
            );
            for achievement in meta_rules::ACHIEVEMENTS {
                let current = meta.0.measure(achievement.measure);
                let unlocked = meta.0.unlocked(achievement);
                let claimed = meta.0.reward_claimed(achievement.id);
                claimable |= unlocked && !claimed;
                let status = if claimed {
                    " · réclamé"
                } else if unlocked {
                    " · disponible"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "\n\n{}{}\n{}\n{} / {} · récompense : {} or",
                    achievement.title,
                    status,
                    achievement.detail,
                    current.min(achievement.target),
                    achievement.target,
                    achievement.reward_gold
                ));
            }
            if tab == 5 && claimable {
                out.push_str("\n\nCliquez sur « Réclamer les récompenses disponibles » pour créditer votre compte.");
            }
            out
        }
        _ => String::new(),
    };
    let title = [
        "Personnage",
        "Collection",
        "Butin",
        "Codex",
        "Hauts faits",
        "Récompenses",
    ][tab.min(5)]
    .to_owned();
    (title, body, claimable)
}

fn kind_name(kind: data::Kind) -> &'static str {
    match kind {
        data::Kind::Bolt => "projectile",
        data::Kind::Strike => "frappe à distance",
        data::Kind::Melee => "attaque de mêlée",
        data::Kind::Nova => "zone autour de vous",
        data::Kind::Cone => "cône frontal",
        data::Kind::Ground | data::Kind::GroundSelf => "zone persistante",
        data::Kind::Chain => "enchaînement",
        data::Kind::Dot => "dégâts périodiques",
        data::Kind::Aura => "aura automatique",
        data::Kind::Orbit => "attaque orbitale",
        data::Kind::Shield => "bouclier",
        data::Kind::Immune => "immunité",
        data::Kind::Thorns => "renvoi",
        data::Kind::Heal => "soin",
        data::Kind::Totem => "totem",
    }
}
