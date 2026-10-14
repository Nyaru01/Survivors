// These are the client's own windows and supporting templates, never the shipped era layer.
pub(super) const FILES: &[&str] = &[
    "Fonts.xml",
    "BasicControls.xml",
    "LocaleProperties.lua",
    "UIParent.xml",
    "MoneyFrame.lua",
    "MoneyFrame.xml",
    "GameTooltip.xml",
    "UIDropDownMenu.xml",
    "UIPanelTemplates.lua",
    "UIPanelTemplates.xml",
    "OptionsFrameTemplates.xml",
    "CharacterFrameTemplates.xml",
    "StaticPopup.xml",
    "ItemButtonTemplate.xml",
    "StackSplitFrame.xml",
    "GameMenuFrame.xml",
    "OptionsFrame.xml",
    "SoundOptionsFrame.xml",
    "UIOptionsFrame.xml",
    "MerchantFrame.xml",
    "UIErrorsFrame.xml",
    "UIMenu.xml",
    "TextStatusBar.lua",
    "TextStatusBar.xml",
    "MainMenuBar.xml",
    "MainMenuBarMicroButtons.xml",
    "Cooldown.xml",
    "ActionButtonTemplate.xml",
    "SpellBookFrame.xml",
    "ActionBarFrame.xml",
    "CombatFeedback.xml",
    "UnitPopup.xml",
    "UnitFrame.xml",
    "PlayerFrame.xml",
    "PetFrame.xml",
    "BuffFrame.xml",
    "PartyFrame.xml",
    "CharacterFrame.xml",
    "PaperDollFrame.xml",
    "PetPaperDollFrame.xml",
    "SkillFrame.xml",
    "ReputationFrame.xml",
    "HonorFrame.xml",
    "ChatFrame.xml",
    "FloatingChatFrame.xml",
    "MainMenuBarBagButtons.xml",
    "ContainerFrame.xml",
    "BankFrame.xml",
    "BonusActionBarFrame.xml",
];

// A targeted UI has no chat, raid frames or stock action bar. Their callbacks are detached,
// not their layout silently replaced with generic panels. ShowUIPanel still owns the windows.
pub(super) const OFFLINE_CALLBACKS: &str = r#"
    UIParent:SetScript("OnUpdate", nil)
    UIParent:UnregisterAllEvents()
    function UIParent_ManageFramePositions() end
    function UpdateMicroButtons() end
    function Disable_BagButtons() end
    function Enable_BagButtons() end
    CloseAllBags = CloseAllBags or function() end
    OpenBackpack = OpenBackpack or function() end
    CloseBackpack = CloseBackpack or function() end
    function UpdateNameplates() end
    function BuffButtons_UpdatePositions() end
    function FCF_Set_SimpleChat() end
    function FCF_Set_NormalChat() end
    function FCF_Set_ChatLocked() end
    function SetChatMouseOverDelay() end
    function MultiActionBar_ShowAllGrids() end
    function MultiActionBar_HideAllGrids() end
    function RaidOptionsFrame_UpdatePartyFrames() end
    function UpdatePartyMemberBackground() end
    function MultiBar1_IsVisible() return nil end
    function MultiBar2_IsVisible() return nil end
    function MultiBar3_IsVisible() return nil end
    function MultiBar4_IsVisible() return nil end
    function MultibarGrid_IsVisible() return nil end
    function Multibar_EmptyFunc() end
    function MultiActionBar_Update() end
    NUM_CONTAINER_FRAMES = NUM_CONTAINER_FRAMES or 0
    CONTAINER_OFFSET_X = CONTAINER_OFFSET_X or 0
    CONTAINER_OFFSET_Y = CONTAINER_OFFSET_Y or 70
    PETACTIONBAR_YPOS = PETACTIONBAR_YPOS or 98
    PETACTIONBAR_XPOS = PETACTIONBAR_XPOS or 36
"#;

pub(super) fn load_windows(
    script: &benilla_ui::script::UiScript,
    read: &impl Fn(&str) -> Option<Vec<u8>>,
) -> Vec<String> {
    let mut errors = Vec::new();
    // GlobalStrings is installed by the boot VM; source::decode preserves the FR CP1252 bytes.
    if let Some(bytes) = read(r"Interface\FrameXML\GlobalStrings.lua") {
        if let Err(e) = script.run_chunk_named(&bytes, "@Interface\\FrameXML\\GlobalStrings.lua") {
            errors.push(e.to_string());
        }
    }
    let Some(toc) = read(r"Interface\FrameXML\FrameXML.toc") else {
        errors.push("FrameXML.toc absent de la chaîne de données.".into());
        return errors;
    };
    let toc = benilla_ui::toc::Toc::parse(&benilla_ui::source::decode(&toc));
    // Order is taken from the supplied client, so localization and template precedence survive.
    for file in toc
        .files
        .iter()
        .filter(|f| FILES.iter().any(|want| f.eq_ignore_ascii_case(want)))
    {
        let path = format!(r"Interface\FrameXML\{file}");
        let Some(bytes) = read(&path) else {
            errors.push(format!("{path} absent"));
            continue;
        };
        if path.ends_with(".lua") {
            if let Err(e) = script.run_chunk_named(&bytes, &format!("@{path}")) {
                errors.push(format!("{path}: {e}"));
            }
        } else {
            match benilla_ui::framexml::parse(&benilla_ui::source::decode(&bytes)) {
                Ok(doc) => {
                    let report =
                        benilla_ui::loader::load_in(script, &doc, &path.replace('\\', "/"), &read);
                    errors.extend(report.errors);
                    errors.extend(
                        report
                            .warnings
                            .into_iter()
                            .filter(|w| w.contains("unknown template")),
                    );
                }
                Err(e) => errors.push(format!("{path}: {e}")),
            }
        }
        // GameMenuFrame sources UIParent.lua again. Reapply the serverless callbacks after both.
        if file.eq_ignore_ascii_case("UIParent.xml")
            || file.eq_ignore_ascii_case("GameMenuFrame.xml")
        {
            if let Err(e) = script.run(OFFLINE_CALLBACKS) {
                errors.push(e.to_string());
            }
        }
    }

    if let Err(e) = script.run(OFFLINE_CALLBACKS) {
        errors.push(e.to_string());
    }
    if let Err(e) = script.run(r#"
        for _, name in ipairs({"MainMenuBar", "PlayerFrame", "PetFrame", "BuffFrame", "ChatFrame1", "ChatFrame2", "ChatFrame3", "ChatFrame4", "ChatFrame5", "ChatFrame6", "ChatFrame7", "PartyMemberFrame1", "PartyMemberFrame2", "PartyMemberFrame3", "PartyMemberFrame4", "PartyMemberBackground"}) do
            local f = getglobal(name)
            if f then f:UnregisterAllEvents() f:Hide() end
        end
    "#) { errors.push(e.to_string()); }
    if let Some(bytes) = read(r"Interface\FrameXML\Bindings.xml") {
        match benilla_ui::bindings_xml::parse(&benilla_ui::source::decode(&bytes)) {
            Ok(bindings) => script.register_bindings(&bindings),
            Err(e) => errors.push(format!("Bindings.xml: {e}")),
        }
    }
    // The portable save stores equipped tiers and one potion stack, not arbitrary bag layout.
    // Keep the stock right-click/use handler while disallowing drag and split until saved slots exist.
    if let Err(e) = script.run(
        r#"
        local original = ContainerFrameItemButton_OnClick
        function ContainerFrameItemButton_OnClick(button)
            if button == "RightButton" then return original(button) end
        end
    "#,
    ) {
        errors.push(e.to_string());
    }
    // The original menu loads this Blizzard addon on demand. Seat the genuine addon directly
    // because this offline VM deliberately has no player-addon discovery/network session.
    let path = r"Interface\AddOns\Blizzard_BindingUI\Blizzard_BindingUI.xml";
    if let Some(bytes) = read(path) {
        match benilla_ui::framexml::parse(&benilla_ui::source::decode(&bytes)) {
            Ok(doc) => {
                let report =
                    benilla_ui::loader::load_in(script, &doc, &path.replace('\\', "/"), read);
                errors.extend(report.errors);
                errors.extend(
                    report
                        .warnings
                        .into_iter()
                        .filter(|w| w.contains("unknown template")),
                );
            }
            Err(e) => errors.push(format!("{path}: {e}")),
        }
        if let Some(bytes) = read(r"Interface\AddOns\Blizzard_BindingUI\Localization.lua") {
            if let Err(e) = script.run_chunk_named(&bytes, "@Blizzard_BindingUI/Localization.lua") {
                errors.push(e.to_string());
            }
        }
        if let Err(e) = script.run("function KeyBindingFrame_LoadUI() end") {
            errors.push(e.to_string());
        }
    } else {
        errors.push(format!("{path} absent"));
    }
    // This Blizzard addon is normally loaded on demand; the offline VM seats it explicitly.
    let talent_path = r"Interface\AddOns\Blizzard_TalentUI\Blizzard_TalentUI.xml";
    if let Some(bytes) = read(talent_path) {
        match benilla_ui::framexml::parse(&benilla_ui::source::decode(&bytes)) {
            Ok(doc) => {
                let report = benilla_ui::loader::load_in(
                    script,
                    &doc,
                    &talent_path.replace('\\', "/"),
                    read,
                );
                errors.extend(report.errors);
                errors.extend(
                    report
                        .warnings
                        .into_iter()
                        .filter(|w| w.contains("unknown template")),
                );
            }
            Err(error) => errors.push(format!("{talent_path}: {error}")),
        }
        if let Err(error) = script.run("function TalentFrame_LoadUI() end") {
            errors.push(error.to_string());
        }
    } else {
        errors.push(format!("{talent_path} absent"));
    }
    for name in [
        "MerchantFrame",
        "GameMenuFrame",
        "OptionsFrame",
        "SoundOptionsFrame",
        "UIOptionsFrame",
        "KeyBindingFrame",
        "TalentFrame",
        "SpellBookFrame",
    ] {
        if !script
            .eval::<bool>(&format!("return {name} ~= nil"))
            .unwrap_or(false)
        {
            errors.push(format!("Fenêtre Blizzard {name} absente après chargement."));
        }
    }
    errors
}
