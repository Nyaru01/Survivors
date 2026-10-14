//! Survivors-only authoring; models are enumerated from the player's local DBCs.
use super::{
    hero::Hero,
    newhub_layout_rules::{Document, Portal, Pose},
    newhub_sites,
    prep::camp::{self, CampEntity},
    Phase, Selection,
};
use benilla_assets::{LockRecover, WorldAssets};
use benilla_ui::script::UiScript;
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub(crate) struct CampActor(pub usize);
#[derive(Component, Clone, Copy)]
pub(crate) struct PortalDestination(pub Option<usize>);
#[derive(Resource)]
pub(crate) struct HubEditor {
    pub active: bool,
    pub revision: u64,
    pub document: Document,
    selected: usize,
    status: String,
    session: u64,
    catalog: Vec<(u32, String, bool)>,
    page: usize,
    search: String,
}
impl Default for HubEditor {
    fn default() -> Self {
        let document = crate::local_state::home()
            .and_then(|h| crate::local_state::read_to_string(&h.join("survivors-hubs.txt")).ok())
            .and_then(|s| Document::decode(&s))
            .unwrap_or_default();
        Self {
            active: false,
            revision: 0,
            document,
            selected: 0,
            status: String::new(),
            session: 0,
            catalog: Vec::new(),
            page: 0,
            search: String::new(),
        }
    }
}
pub(crate) fn faction(site: &newhub_sites::FactionHub) -> usize {
    usize::from(site.faction == newhub_sites::HubFaction::Horde)
}
impl HubEditor {
    pub(crate) fn transform(&self, site: &newhub_sites::FactionHub, actor: usize) -> Transform {
        let h = &self.document.hubs[faction(site)];
        let pose = if actor < 2 {
            h.npcs[actor].pose
        } else {
            h.portals.get(actor - 2).and_then(|p| p.pose)
        };
        let loc = match actor {
            0 => site.merchant.location,
            1 => site.artisan.location,
            _ => site.portal.anchor,
        };
        let mut xyz = super::units::wow_to_bevy(loc.xyz);
        if actor > 2 {
            xyz.x += (actor - 2) as f32 * 3.0;
        }
        let origin = super::units::wow_to_bevy(site.arrival.xyz);
        let p = pose
            .filter(|p| Vec3::from_array(p.xyz).distance(origin) <= 90.0)
            .unwrap_or(Pose {
                xyz: xyz.to_array(),
                yaw: if actor < 2 { loc.orientation } else { 0.0 },
            });
        Transform::from_translation(Vec3::from_array(p.xyz))
            .with_rotation(Quat::from_rotation_y(p.yaw))
    }
    fn save(&mut self) {
        self.status = match crate::local_state::home() {
            Some(h) => {
                let path = h.join("survivors-hubs.txt");
                let backup = crate::local_state::read_to_string(&path)
                    .ok()
                    .map(|old| {
                        crate::local_state::write_atomic(&h.join("survivors-hubs.bak"), &old)
                    })
                    .transpose();
                match backup
                    .and_then(|_| crate::local_state::write_atomic(&path, &self.document.encode()))
                {
                    Ok(()) => "Camp sauvegardé.".into(),
                    Err(e) => format!("Échec : {e}"),
                }
            }
            None => "Sauvegarde indisponible.".into(),
        };
    }
}
const PANEL: &str = r#"
if not SurvivorsHubEditor then
 local f=CreateFrame('Frame','SurvivorsHubEditor',UIParent)
 f:SetWidth(430); f:SetHeight(590); f:SetPoint('RIGHT',UIParent,'RIGHT',-16,0); f:SetFrameStrata('DIALOG')
 f:SetBackdrop({bgFile='Interface\\DialogFrame\\UI-DialogBox-Background',edgeFile='Interface\\DialogFrame\\UI-DialogBox-Border',tile=true,tileSize=32,edgeSize=32,insets={left=11,right=12,top=12,bottom=11}})
 local title=f:CreateFontString(nil,'OVERLAY','GameFontNormalLarge'); title:SetPoint('TOP',f,'TOP',0,-16); title:SetText('Aménagement du camp')
 local c=CreateFrame('Button',nil,f,'UIPanelCloseButton'); c:SetPoint('TOPRIGHT',f,'TOPRIGHT',-4,-4); c:SetScript('OnClick',function() __hub_close=true end)
 local function label(name,y,text)
  local t=f:CreateFontString(name,'OVERLAY','GameFontHighlightSmall'); t:SetPoint('TOPLEFT',f,'TOPLEFT',18,y); t:SetWidth(394); t:SetJustifyH('LEFT'); t:SetText(text)
 end
 local function button(name,text,x,y,w,action)
  local b=CreateFrame('Button',name,f,'UIPanelButtonTemplate'); b:SetWidth(w); b:SetHeight(25); b:SetPoint('TOPLEFT',f,'TOPLEFT',x,y); b:SetText(text); b:SetScript('OnClick',action)
 end
 label('SurvivorsHubSelected',-50,'Marchand')
 button(nil,'Objet précédent',18,-74,190,function() __hub_select=-1 end)
 button(nil,'Objet suivant',218,-74,190,function() __hub_select=1 end)
 button(nil,'Ajouter un portail',18,-104,190,function() __hub_add=true end)
 button(nil,'Retirer le portail',218,-104,190,function() __hub_remove=true end)
 local function edit(name,y)
  local e=CreateFrame('EditBox',name,f,'InputBoxTemplate'); e:SetWidth(365); e:SetHeight(24); e:SetPoint('TOPLEFT',f,'TOPLEFT',28,y); e:SetAutoFocus(false); e:SetMaxLetters(48)
  e:SetScript('OnEnterPressed',function() e:ClearFocus() end); e:SetScript('OnEscapePressed',function() e:ClearFocus() end); return e
 end
 label(nil,-141,'Nom du portail')
 local portalName=edit('SurvivorsHubName',-161)
 portalName:SetScript('OnEditFocusGained',function() __hub_name_focused=true end)
 portalName:SetScript('OnEditFocusLost',function() __hub_name_focused=false end)
 portalName:SetScript('OnEnterPressed',function() __hub_name=portalName:GetText(); portalName:ClearFocus() end)
 button(nil,'Renommer',18,-189,120,function() __hub_name=SurvivorsHubName:GetText() end)
 label('SurvivorsHubDestination',-222,'Destination')
 button(nil,'Monde précédent',18,-249,190,function() __hub_world=-1 end)
 button(nil,'Monde suivant',218,-249,190,function() __hub_world=1 end)
 label(nil,-283,'Modèles du client — filtre : Human, Troll, Portal…')
 local search=edit('SurvivorsHubSearch',-303)
 search:SetScript('OnTextChanged',function() __hub_filter=search:GetText() end)
 for i=1,6 do local n=i; button('SurvivorsHubModel'..i,'',18,-333-(i-1)*25,390,function() __hub_model=n end) end
 button(nil,'< Modèles',18,-488,125,function() __hub_page=-1 end)
 button(nil,'Modèles >',153,-488,125,function() __hub_page=1 end)
 button(nil,'Taille −',288,-488,58,function() __hub_scale=-1 end)
 button(nil,'Taille +',350,-488,58,function() __hub_scale=1 end)
 label('SurvivorsHubStatus',-521,'')
 button(nil,'Sauvegarder (F9)',18,-556,190,function() __hub_save=true end)
 button(nil,'Fermer (F8)',218,-556,190,function() __hub_close=true end)
 f:Hide()
end
"#;
#[derive(Component)]
struct EditorHelp;
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HubEditor>()
        .add_systems(Startup, help)
        .add_systems(
            Update,
            edit.after(camp::walk).after(crate::ui_script::UiInput),
        );
}
fn help(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        EditorHelp,
        Text::new(""),
        TextFont {
            font: assets.load("mpq://Fonts/FRIZQT__.ttf"),
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.82, 0.0)),
        TextShadow::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            top: Val::Px(130.0),
            max_width: Val::Px(510.0),
            padding: UiRect::all(Val::Px(12.0)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        GlobalZIndex(1500),
    ));
}
fn flag(script: &UiScript, name: &str) -> bool {
    let g = script.lua().globals();
    let v = g.get::<bool>(name).unwrap_or(false);
    let _ = g.set(name, false);
    v
}
fn number(script: &UiScript, name: &str) -> i32 {
    let g = script.lua().globals();
    let v = g.get::<i32>(name).unwrap_or(0);
    let _ = g.set(name, 0);
    v
}
#[allow(clippy::too_many_arguments)]
fn edit(
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    selection: Res<Selection>,
    settings: Res<super::settings::Settings>,
    stock: Res<super::blizzard_ui::StockUi>,
    keyboard: Res<crate::ui_script::UiKeyboardCapture>,
    assets: Option<Res<WorldAssets>>,
    mut editor: ResMut<HubEditor>,
    hero: Query<&Transform, With<Hero>>,
    mut actors: Query<(&CampActor, &mut Transform), (With<CampEntity>, Without<Hero>)>,
    mut help: Query<(&mut Text, &mut Node), With<EditorHelp>>,
    script: Option<NonSendMut<UiScript>>,
    progress: Res<super::character_save::WorldProgressStore>,
) {
    let preparing = *phase.get() == Phase::Preparing;
    if !preparing {
        editor.active = false;
    }
    if preparing && !settings.open && !stock.menu_requested && keys.just_pressed(KeyCode::F8) {
        editor.active = !editor.active;
    }
    let Some(mut script) = script else { return };
    if !stock.ready {
        return;
    }
    if editor.session != script.session() {
        if let Err(e) = script.run(PANEL) {
            warn!("Hub editor: {e}");
            return;
        }
        editor.session = script.session();
    }
    if flag(&script, "__hub_close") {
        editor.active = false;
    }
    let _ = script.run(if editor.active && preparing {
        "SurvivorsHubEditor:Show()"
    } else {
        "SurvivorsHubEditor:Hide()"
    });
    let site = newhub_sites::for_race(super::menu::selected_race(&selection).unwrap_or(1));
    let fi = faction(site);
    if editor.active && editor.catalog.is_empty() {
        if let Some(assets) = assets {
            let mut chain = assets.chain.lock_recover();
            if let Ok(c) = benilla_formats::load_gameobject_catalog(&mut chain) {
                editor.catalog.extend(
                    c.iter()
                        .filter(|(_, p)| {
                            p.to_ascii_lowercase().contains("portal")
                                && !p.to_ascii_lowercase().ends_with(".wmo")
                        })
                        .map(|(id, p)| (id, p.to_owned(), true)),
                );
            }
            if let Ok(c) = benilla_formats::load_creature_catalog(&mut chain) {
                editor
                    .catalog
                    .extend(c.display_models().filter_map(|(id, _)| {
                        c.model(id)
                            .filter(|m| m.scale > 0.0 && m.scale.is_finite())
                            .map(|m| (id, m.model_path, false))
                    }));
            }
            editor
                .catalog
                .sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        }
    }
    if editor.active {
        let delta = number(&script, "__hub_select")
            + i32::from(!keyboard.typing && keys.just_pressed(KeyCode::F6));
        if delta != 0 {
            let count = editor.document.hubs[fi].portals.len() + 2;
            editor.selected = (editor.selected as i32 + delta).rem_euclid(count as i32) as usize;
            editor.page = 0;
        }
        if flag(&script, "__hub_add") && editor.document.hubs[fi].portals.len() < 64 {
            let pose = hero.single().ok().map(|t| Pose {
                xyz: t.translation.to_array(),
                yaw: t.rotation.to_euler(EulerRot::YXZ).0,
            });
            let n = editor.document.hubs[fi].portals.len() + 1;
            editor.document.hubs[fi].portals.push(Portal {
                name: format!("Portail {n}"),
                destination: Some(selection.map),
                pose,
                ..Portal::default()
            });
            editor.selected = n + 1;
            editor.revision += 1;
        }
        if flag(&script, "__hub_remove")
            && editor.selected >= 2
            && editor.document.hubs[fi].portals.len() > 1
        {
            let s = editor.selected;
            editor.document.hubs[fi].portals.remove(s - 2);
            editor.selected = 2;
            editor.revision += 1;
        }
        editor.selected = editor
            .selected
            .min(editor.document.hubs[fi].portals.len() + 1);
        let selected = editor.selected;
        if let Ok(name) = script.lua().globals().get::<String>("__hub_name") {
            let _ = script
                .lua()
                .globals()
                .set("__hub_name", Option::<String>::None);
            let name = name.trim();
            if selected >= 2
                && !name.is_empty()
                && name.chars().count() <= 48
                && !name.chars().any(char::is_control)
            {
                editor.document.hubs[fi].portals[selected - 2].name = name.into();
                editor.revision += 1;
            }
        }
        let world = number(&script, "__hub_world");
        if selected >= 2 && world != 0 {
            let p = &mut editor.document.hubs[fi].portals[selected - 2];
            p.destination = Some(
                (p.destination.unwrap_or(selection.map) as i32 + world)
                    .rem_euclid(super::data::MAPS.len() as i32) as usize,
            );
            editor.revision += 1;
        }
        for (actor, mut tf) in &mut actors {
            if actor.0 != selected || keyboard.typing {
                continue;
            }
            let mut changed = false;
            if keys.just_pressed(KeyCode::F7) {
                if let Ok(p) = hero.single() {
                    if p.translation
                        .distance(super::units::wow_to_bevy(site.arrival.xyz))
                        <= 90.0
                    {
                        tf.translation = p.translation;
                        tf.rotation = p.rotation;
                        changed = true;
                    }
                }
            }
            let turn = i32::from(keys.just_pressed(KeyCode::PageUp))
                - i32::from(keys.just_pressed(KeyCode::PageDown));
            if turn != 0 {
                tf.rotate_y(turn as f32 * std::f32::consts::PI / 12.0);
                changed = true;
            }
            if changed {
                let pose = Some(Pose {
                    xyz: tf.translation.to_array(),
                    yaw: tf.rotation.to_euler(EulerRot::YXZ).0,
                });
                if selected < 2 {
                    editor.document.hubs[fi].npcs[selected].pose = pose;
                } else {
                    editor.document.hubs[fi].portals[selected - 2].pose = pose;
                }
                editor.status = "Placement modifié : F9 pour conserver.".into();
            }
            if keys.just_pressed(KeyCode::Delete) {
                if selected < 2 {
                    editor.document.hubs[fi].npcs[selected].pose = None;
                } else {
                    editor.document.hubs[fi].portals[selected - 2].pose = None;
                }
                let original = editor.transform(site, selected);
                tf.translation = original.translation;
                tf.rotation = original.rotation;
            }
        }
        if let Ok(s) = script.lua().globals().get::<String>("__hub_filter") {
            if editor.search != s {
                editor.search = s;
                editor.page = 0;
            }
        }
        let delta = number(&script, "__hub_page");
        let filter = editor.search.to_ascii_lowercase();
        let models: Vec<_> = editor
            .catalog
            .iter()
            .filter(|(id, path, p)| {
                *p == (selected >= 2)
                    && (path.to_ascii_lowercase().contains(&filter)
                        || id.to_string().contains(&filter))
            })
            .cloned()
            .collect();
        let pages = models.len().max(1).div_ceil(6);
        editor.page =
            (editor.page as i32 + delta).clamp(0, pages.saturating_sub(1) as i32) as usize;
        let model = number(&script, "__hub_model");
        if model > 0 {
            if let Some((id, path, _)) = models.get(editor.page * 6 + (model - 1) as usize) {
                if selected < 2 {
                    editor.document.hubs[fi].displays[selected] = *id;
                } else {
                    let p = &mut editor.document.hubs[fi].portals[selected - 2];
                    p.display = *id;
                    p.scale = if path.to_ascii_lowercase().contains("mageportals\\") {
                        0.1
                    } else {
                        1.0
                    };
                }
                editor.revision += 1;
            }
        }
        let scale = number(&script, "__hub_scale");
        if selected >= 2 && scale != 0 {
            let p = &mut editor.document.hubs[fi].portals[selected - 2];
            p.scale = (p.scale * if scale > 0 { 1.2 } else { 1.0 / 1.2 }).clamp(0.05, 3.0);
            editor.revision += 1;
        }
        let hub = &editor.document.hubs[fi];
        let current = if selected < 2 {
            format!(
                "{} — modèle {}",
                ["Marchand", "Artisan"][selected],
                hub.displays[selected]
            )
        } else {
            format!(
                "{} — modèle {}",
                hub.portals[selected - 2].name,
                hub.portals[selected - 2].display
            )
        };
        let g = script.lua().globals();
        let _ = g.set("__hub_current", current);
        let dest = if selected >= 2 {
            let m = hub.portals[selected - 2]
                .destination
                .unwrap_or(selection.map);
            let zone = super::localization::zone_name(super::data::MAPS[m].name)
                .unwrap_or(super::data::MAPS[m].name);
            format!(
                "Destination : {}{}",
                zone,
                if progress
                    .0
                    .available(super::menu::selected_race(&selection).unwrap_or(1), m)
                {
                    ""
                } else {
                    " (verrouillé)"
                }
            )
        } else {
            "Choisissez un portail pour régler sa destination.".into()
        };
        let _ = g.set("__hub_destination", dest);
        let _ = g.set(
            "__hub_portal_name",
            if selected >= 2 {
                hub.portals[selected - 2].name.clone()
            } else {
                String::new()
            },
        );
        for i in 0..6 {
            let label = models
                .get(editor.page * 6 + i)
                .map(|(id, p, _)| format!("{} — {}", id, p.rsplit('\\').next().unwrap_or(p)))
                .unwrap_or_default();
            let _ = g.set(format!("__hub_row{}", i + 1), label);
        }
        if flag(&script, "__hub_save") || (!keyboard.typing && keys.just_pressed(KeyCode::F9)) {
            editor.save();
        }
        let _ = script
            .lua()
            .globals()
            .set("__hub_status", editor.status.clone());
        let _=script.run("SurvivorsHubSelected:SetText(__hub_current); SurvivorsHubDestination:SetText(__hub_destination); SurvivorsHubStatus:SetText(__hub_status); if not __hub_name_focused then SurvivorsHubName:SetText(__hub_portal_name) end; for i=1,6 do getglobal('SurvivorsHubModel'..i):SetText(getglobal('__hub_row'..i)); end");
    }
    for (mut text, mut node) in &mut help {
        node.display = if preparing && !settings.open {
            Display::Flex
        } else {
            Display::None
        };
        text.0 = if editor.active {
            "F6 : choisir · F7 : poser à vos pieds\nPage précédente/suivante : tourner\nSuppr : origine · F9 : sauvegarder · F8 : fermer".into()
        } else {
            "F8 : aménager le camp (PNJ et portails)".into()
        };
    }
}
