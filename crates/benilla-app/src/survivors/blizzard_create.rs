//! Offline adapter for the existing Blizzard character-creation screen.
//! Appearance changes never clear the account, inventory or suspended adventure.
use super::{data, hud, menu, Selection};
use benilla_protocol::CharCreateReq;
use bevy::prelude::*;

#[derive(States, Default, Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum CreateScreen {
    #[default]
    Closed,
    Creating,
}

#[derive(Resource, Default)]
pub(crate) struct LocalCreate {
    pub active: bool,
    pub seed: Option<CharCreateReq>,
    pub accepted: Option<CharCreateReq>,
    pub cancelled: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.init_state::<CreateScreen>()
        .init_resource::<LocalCreate>()
        .add_systems(OnEnter(CreateScreen::Closed), deactivate)
        .add_systems(Update, finish.before(menu::menu_clicks));
    crate::char_create::local_create_plugin(app);
}

fn deactivate(mut local: ResMut<LocalCreate>) {
    local.active = false;
}

pub(super) fn open(
    selection: &Selection,
    local: &mut LocalCreate,
    next: &mut NextState<CreateScreen>,
) {
    let Some(&(_, race, sex)) = selection.look.and_then(|i| selection.looks.get(i)) else {
        return;
    };
    let Some(class) = selection.class_def() else {
        return;
    };
    let [skin, face, hair_style, hair_color, facial_hair] = selection.appearance;
    local.seed = Some(CharCreateReq {
        name: selection.name.clone(),
        race,
        class: class.id,
        gender: sex,
        skin,
        face,
        hair_style,
        hair_color,
        facial_hair,
    });
    local.accepted = None;
    local.cancelled = false;
    local.active = true;
    next.set(CreateScreen::Creating);
}

fn finish(
    mut local: ResMut<LocalCreate>,
    mut next: ResMut<NextState<CreateScreen>>,
    mut selection: ResMut<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
    catalog: Option<Res<crate::entities::CharCreate>>,
    mut saved: ResMut<super::character_save::CharacterSave>,
) {
    if !local.active {
        return;
    }
    if local.cancelled {
        local.cancelled = false;
        next.set(CreateScreen::Closed);
        return;
    }
    let Some(request) = local.accepted.take() else {
        return;
    };
    let (Some(creatures), Some(catalog)) = (creatures.as_deref(), catalog.as_deref()) else {
        return;
    };
    let Some(class) = data::CLASSES.iter().position(|c| c.id == request.class) else {
        return;
    };
    let looks = hud::looks_for(&data::CLASSES[class], creatures);
    let Some(index) = looks
        .iter()
        .position(|&(_, r, s)| r == request.race && s == request.gender)
    else {
        saved.notice = "Cette combinaison n’est pas disponible dans les données du client.".into();
        return;
    };
    selection.name = request.name;
    selection.class = Some(class);
    selection.looks = looks;
    selection.look = Some(index);
    selection.appearance = [
        request.skin,
        request.face,
        request.hair_style,
        request.hair_color,
        request.facial_hair,
    ];
    super::character_save::clamp(&mut selection, catalog);
    selection.version += 1;
    saved.editing_name = false;
    saved.notice = "Personnage enregistré. Votre partie sauvegardée est conservée.".into();
    next.set(CreateScreen::Closed);
}
