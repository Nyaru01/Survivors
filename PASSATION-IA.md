# Passation IA — Azeroth Survivors

État au **14 octobre 2026**. Référence du code et de l’EXE livré : **`0df2df9`** (`Add source-backed Vanilla preparation hubs and shared interactions`). Ce document est un instantané : vérifier le Git et les fichiers avant de reprendre.

## Commencer ici

1. Lire `AGENTS.md`, `docs/METHOD.md`, `docs/MAP.md` et `docs/CONTRIBUTING.md` avant de modifier le client.
2. Lire `CONTROLS.md` et `docs/SURVIVORS-HUBS.md` pour le comportement actuel.
3. Inspecter `git status --short`, la branche et les changements existants. Ne jamais écraser le travail en cours.
4. Préserver les sauvegardes et vérifier le problème dans une vraie exécution avant d’annoncer une correction.

Le projet est un jeu autonome Rust + Bevy basé sur benilla/wenilla, avec des modules Survivors. Il ne s’exécute pas à l’intérieur du client officiel WoW Forever. Le package Windows est `warcraft-survivors`.

## Dépôt, livraison et données

| Élément | Emplacement |
| --- | --- |
| Projet | `D:\VibeCoding\WoW survivors` |
| Git utilisateur | `https://github.com/Nyaru01/Survivors.git` |
| Branche locale au dernier contrôle | `fix/portraits-camera` |
| Branche distante de livraison | `build/windows-exe` |
| PR existante, brouillon | `https://github.com/Nyaru01/Survivors/pull/1` |
| EXE livré | `out\AzerothSurvivors\AzerothSurvivors.exe` |
| Chaîne MPQ FR utilisée | `out\AzerothSurvivors\Data-frFR` |
| Sauvegarde réelle | `out\AzerothSurvivors\benilla-config\survivors-save.toml` |
| Outils, compilation, preuves | `D:\SurvivorsTools` |

Clients fournis par l’utilisateur : `D:\VibeCoding\WoW survivors\1.12.1\World of Warcraft` et `D:\WoW Forever\World of Warcraft\_classic_beta_`. Les conserver intacts. La chaîne runtime validée est celle de `Data-frFR` (Vanilla 1.12.1 build 5875) ; ne pas remplacer ses données par celles du client moderne.

Ne pas committer les MPQ, modèles, textures, sauvegardes personnelles ou fichiers Blizzard extraits. Les assets sont lus localement au runtime. Les modifications dans les fichiers upstream doivent suivre les règles de carries de `AGENTS.md` ; privilégier les modules Survivors propres au projet.

Ne pas forcer un push, synchroniser avec « Sync fork », fusionner vers `main` ou changer le dépôt cible par défaut. Les exemples Arnesen dans la documentation upstream ne remplacent pas le Git de Nyaru01.

## Exigences utilisateur à conserver

- Jeu en français, interface Blizzard Vanilla issue des données locales ; **pas EllesmereUI**.
- Sorts propres à la classe, automatiques avec déclenchement manuel possible par clic/raccourci.
- Portraits joueur et cible avec vie et niveau ; barre de sorts lisible, descriptions au survol et recharge correcte.
- Personnage nommé et personnalisable ; talents, équipement permanent, sacs et progression conservés. Reprise de partie en cours souhaitée.
- Retour à la sélection après la mort ; pas de rotation permanente de caméra à la sélection.
- Préparation permettant talents, équipement, grimoire et sacs, puis départ vers un monde autorisé.
- Départ lié à la race ; quatre boss vaincus débloquent le monde suivant. Autres zones de départ masquées, mondes verrouillés grisés.
- Boss toutes les trois minutes même si un boss précédent est encore vivant.
- Or au sol en plus de l’XP ; coffres payants avec bonus temporaires pour la partie/le monde. Boss : équipement permanent conservé dans les sacs.
- Potions de soin en quantité limitée ; progression d’XP initiale moins rapide.
- Camp **3D explorable** avec PNJ marchand, artisan et portail. Fusion : trois blancs identiques donnent un vert amélioré.
- Travailler sur D:, économiser les tokens et utiliser des agents Luna seulement pour des sous-tâches utiles et distinctes.

Ces exigences ne constituent pas une certification manuelle de tous les parcours. Les validations et limites ci-dessous font foi.

## Dernier travail livré : hubs Vanilla

Alliance : Sanctum des Sorciers de Hurlevent, map 0, salle supérieure. Horde : Faille de l’Ombre d’Orgrimmar, map 1. Deux PNJ et un portail utilisables par faction, positions appuyées sur SQL/DBC et triangles de collision WMO.

Arrivées en coordonnées WoW : Alliance `(-9014.94, 873.326, 148.616)` ; Horde `(1800.660, -4374.510, -17.3475)`. Conversion : `units::wow_to_bevy([x,y,z]) = Vec3(-y,z,-x)`.

Les displays PNJ validés sont **191** (marchand) et **243** (artisan). Le portail compact utilise GameObjectDisplayInfo **4396**, template **176296**, échelle 1. Ne pas rétablir le portail display 1167 : son modèle mesure 31,7 yards de haut. Le display PNJ 19 a une échelle nulle.

Aucun panneau marchand, artisan ou sac ne s’ouvre à l’arrivée. Clic droit à portée ou `E` sur le service. Les boutons Talents, Équipement, Grimoire et Sacs restent disponibles ; pas de boutons globaux remplaçant les PNJ.

La fusion valide trois objets blancs identiques, non équipés, avant toute mutation. Elle consomme exactement trois instances et crée un vert **de rang supérieur**, plafond 8. Les statistiques dépendent du rang : changer seulement la couleur serait une régression. Identité du template : emplacement, qualité et rang ; les IDs sont propres à Survivors, pas ceux d’Item.dbc Vanilla. Les ingrédients partiels 1/3 et 2/3 sont affichés sans être consommés.

## Carte du code

Chemin de base : `crates/benilla-app/src/survivors/`.

| Fichier | Responsabilité |
| --- | --- |
| `newhub_sites.rs` | Lieux, sources, positions et modèles des services |
| `camp.rs` | Entités du camp, déplacement avec collisions/sol, noms des services |
| `newinteract.rs` | Interaction commune PNJ, portail et coffres ; portée et occlusion |
| `hero.rs` | Routage des cartes selon la phase, caméra et suivi du personnage |
| `hud.rs` | Interface et apparition du personnage de préparation au bon hub |
| `prep.rs` | Talents, équipement, grimoire, sacs et interface de fusion |
| `blizzard_ui.rs` | Adaptations offline FrameXML, marchand, inventaire et aperçu de fusion |
| `economy.rs` | Réception des interactions/coffres et protection contre double ouverture |
| `economy_rules.rs` | Règles économiques, identité des objets, recettes |
| `newhub_verify.rs` | Sonde native isolée des deux hubs |
| `journey_verify.rs` | Ancienne sonde de parcours ; pas la référence validée du nouveau hub |
| `mod.rs` | Enregistrement des plugins Survivors |

Autres repères : `crates/benilla-app/src/cursor.rs`, `crates/benilla-app/tests/survivors_economy.rs`, `tests/survivors-rules/`, `scripts/build-windows.ps1`.

### Régressions à éviter

- Le personnage de prévisualisation en phase Preparing doit apparaître au hub : `hud.rs` le recréait auparavant dans l’arène, produisant des captures de forêt alors que les sondes PNJ passaient.
- Ne pas appliquer la contrainte de déplacement de l’arène au camp. La phase de préparation utilise son propre sol/collision et son routage de carte.
- La caméra initiale vise la salle depuis l’arrivée vers le portail ; l’orientation SQL brute produisait une vue obstruée par un portail décoratif.
- Le contrôleur normal est ignoré en CaptureMode : préserver le traitement complémentaire du clic droit dans `newinteract.rs`. Il exclut les glissés de plus de 5 px et les pressions/relâchements sur l’UI.
- Portée d’interaction 3D : 3,4 yards entre pieds du joueur et racine de l’acteur, avec occlusion. Ne pas désactiver ces contrôles pour faire passer un test.
- Garder la protection contre deux ouvertures du même coffre dans une frame.
- Un seul panneau métier ouvert à la fois ; Échap/croix ferment. Le marchand ne doit pas ouvrir automatiquement les sacs à l’arrivée.

## Compiler et vérifier

Depuis la racine, PowerShell :

```powershell
& .\scripts\build-windows.ps1
```

Le script prépare les variables d’environnement sur D:, lance `cargo build --locked --release -p warcraft-survivors --no-default-features`, copie l’EXE, les DLL et les documents de distribution. Une recompilation du client peut prendre environ dix minutes : regrouper les changements avant de compiler.

Tests des règles sans démarrer le moteur :

```powershell
$env:RUSTUP_HOME = 'D:\SurvivorsTools\rustup'
$env:CARGO_HOME = 'D:\SurvivorsTools\cargo'
$env:CARGO_TARGET_DIR = 'D:\SurvivorsTools\rules-target'
$env:TEMP = 'D:\SurvivorsTools\tmp'
$env:TMP = $env:TEMP
$env:PATH = "D:\SurvivorsTools\cargo\bin;D:\SurvivorsTools\mingw64\bin;$env:PATH"
cargo test --manifest-path tests/survivors-rules/Cargo.toml --locked
```

Pour formater seulement les fichiers modifiés : `rustfmt --config skip_children=true --edition 2021 <fichiers>`. Éviter de reformater tout le workspace par inadvertance.

Dernière validation : **61 tests de règles** (classes, économie, FR, sauvegarde, talents, mondes, XP), **3 tests mathématiques d’interaction**, compilation Windows et workflow GitHub `check` **38584628580** réussis (wasm, client-carries, host-and-ui).

Preuves natives :

- `D:\SurvivorsTools\vanilla-hubs-delivery-proof\profile\verification-hub.txt` et sous-dossier `shots\` : référence actuelle, `STATUS=PASS`.
- `D:\SurvivorsTools\vanilla-hubs-final-small\profile\verification-hub.txt` et `shots\` : panneaux petite résolution, avant le dernier réglage de caméra, `STATUS=PASS`.
- `D:\SurvivorsTools\hub-recon\rules-final.log` : tests ciblés.
- `D:\SurvivorsTools\tmp\hubs-build-camera.log` et `hubs-build-stamp.log` : compilation et estampillage.

Les sondes couvrent les deux factions, positions/sol/occlusion/portée, achat à 60 or, fusion avec hausse de rang, ouverture via le gestionnaire commun et portail vers le monde autorisé. Captures relues à 1620×690 et 1920×1080. Le hash de la sauvegarde utilisateur est resté identique ; référence avant tests : `D:\SurvivorsTools\vanilla-hubs-proof\user-save-before.sha256`.

### Rejouer la sonde sans toucher aux sauvegardes

Utiliser un **nouveau dossier de profil isolé**, jamais `out\AzerothSurvivors\benilla-config`. Y créer `survivors-probe.marker` contenant exactement `isolated validation`. Définir `BENILLA_HOME` sur ce profil, `WOW_DATA` sur la chaîne `Data-frFR`, `WOW_SURVIVORS_VERIFY=hub` ; supprimer les variables `WOW_SURVIVORS_AUTO` et `WOW_CAPTURE` si présentes. Lancer l’EXE depuis sa distribution. Option : `WOW_SURVIVORS_HUB_PANEL_SMALL=1` pour les panneaux en petite résolution.

Lire le **rapport `STATUS`**, pas uniquement le code de sortie : la sonde peut terminer avec un code succès même en échec. Attendre que les captures PNG soient complètement écrites. Un rapport numérique réussi ne prouve pas que l’image montre le bon lieu.

## Suite à valider manuellement

### Extension après l’éditeur initial

Le code en cours ajoute plusieurs portails nommés et leurs destinations, un catalogue runtime GameObjectDisplayInfo/CreatureDisplayInfo avec filtre/pages et modèles de PNJ, un format de hub v2 avec migration du v1 et sauvegarde de secours. `character_roster.rs` migre la sauvegarde plate vers des personnages indépendants ; les trois sondes qui lisent le disque doivent passer par son décodeur pour extraire le slot actif. Le menu est paginé par quatre personnages. La sélection montre le héros face à la caméra, et les boutons Préparation/Apparence/Reprendre/Options sont à gauche.

`camp.rs` gère un saut sur collisions WMO. `blizzard_ui.rs` conserve les sacs/fiche/talents/grimoire nonmodaux ; ne pas les réintroduire dans `modal_open`, sinon la pause des settings rebloque le mouvement. L’autosave n’efface la reprise qu’en phase GameOver : un `dead_t` résiduel au menu ne doit jamais effacer la partie d’un autre slot.

`graphics_defaults.rs` applique les maxima sur profil sans config, en dehors des tables upstream. `startup_splash.rs` et `loading_hd.rs` embarquent les illustrations originales `assets/survivors/*.png` ; voir README pour provenance, prompts et résolution réelle. Le chargement HD est une couche opaque au-dessus du HUD, conserve l’état de readiness du moteur et cache les étiquettes de camp. La barre utilise WorldLoadProgress, le logo/barre sont lus dans les MPQ. L’illustration du chargement est une réinterprétation, pas une copie exacte du dessin Vanilla (retouche directe rejetée par le filtre d’image).

Tests purs de l’extension : `D:\SurvivorsTools\tmp\hub-expansion-rules.log`. Essai natif de l’éditeur : mode habituel `WOW_SURVIVORS_VERIFY=hub` dans profil isolé avec marqueur, plus `WOW_SURVIVORS_EDITOR_PROBE=1` ; exerce ajout/nom/destination/sauvegarde/retrait puis services des deux factions. Ne jamais utiliser le profil réel dans ces sondes. Vérifier le rapport avant d’annoncer un résultat.

Livraison de l’extension : 80 tests purs et compilation Windows réussis ; sonde native finale `D:\SurvivorsTools\hub-expansion-proof-v2\profile\verification-hub.txt` = `STATUS=PASS`. Captures `loading-hd.png` et `shots/editor-named-portals.png` relues ; chargement sans texte du camp. Activer `WOW_SURVIVORS_HD_LOADING_PROBE=1` pour cette capture dans un profil marqué isolé. Le dossier `hub-expansion-proof` précédent est un essai échoué, ne pas le citer comme succès. Les PNG doivent être décodés par `image::load_from_memory` : Bevy n’active pas sa feature PNG. Vanilla EditBox n’a pas HasFocus : le champ conserve ses flags OnEditFocusGained/Lost. Laisser les transforms se propager après reconstruction des acteurs avant de contrôler leurs positions.

Ajout après l’instantané initial : outil de placement du camp dans `newhub_editor.rs` et codec `newhub_layout_rules.rs`. F8 active, F6 choisit, F7 pose aux pieds, Page précédente/suivante tourne, Suppr remet l’origine et F9 écrit `benilla-config/survivors-hubs.txt`. Les overrides sont propres à chaque faction et séparés des sauvegardes de personnage. Voir `CONTROLS.md` et `docs/SURVIVORS-HUBS.md` ; le commit de référence en tête de ce document décrit la livraison précédente.

1. Clic droit réel sur PNJ/portail, glissés de caméra et priorité de l’UI ; déplacement dans les deux camps.
2. Coffres pendant un combat : clic, coût unique, équipement temporaire et séparation des objets permanents des boss.
3. Quitter puis reprendre une partie existante, conservation du personnage/talents/sacs et parcours mort → sélection → préparation.
4. Examiner le portrait marchand offline si son cercle reste noir ; aucune correction de ce point n’a été certifiée dans la livraison des hubs.

Les sondes envoient des interactions projetées, événements Bevy et callbacks Lua ; elles ne pilotent pas la souris physique. Ne pas annoncer ces parcours manuels comme testés. Les PNJ peuvent être masqués selon l’angle et les décors : ne pas promettre qu’ils sont tous visibles au premier cadrage ni une fidélité parfaite au client Blizzard.

Pour une reprise efficace : commencer par ces essais, traiter les défauts observés, conserver les données personnelles, puis relancer uniquement les validations concernées. Un changement documentaire seul ne nécessite pas de reconstruire l’EXE.

### Sélection et administration — révision du 14 octobre au soir

La liste des personnages est à droite ; Créer ouvre le module natif Blizzard avec race, sexe, classe, apparence et nom. `LocalCreateMode::New` transmet un `pending_create` complet sans modifier la sélection sortante. Apparence reste un mode distinct. La suppression demande une confirmation et utilise l’identifiant stable du slot. Le roster vide est un état sauvegardé valide ; il ne doit pas être réamorcé par l’autosave. Une installation sans sauvegarde conserve l’amorçage initial. Les résumés du roster incluent race, sexe et niveau ; la pagination n’apparaît que lorsqu’elle est utile. Le bouton Mondes affiche la liste des zones à gauche ; elle est masquée au départ.

`character_save::playable_looks` complète les modèles de PNJ historiques par les corps ChrRaces pour toutes les combinaisons autorisées par CharBaseInfo. Ne pas réintroduire la dépendance exclusive à `ClassDef.looks` : elle peut refuser un troll guerrier alors que la création native le permet.

F8 ouvre l’administration du camp. La bibliothèque filtre modèles de PNJ ou portails, catégories, noms et IDs ; cliquer une ligne crée un aperçu 3D temporaire à côté du joueur. `newhub_preview.rs` crée une entité distincte, sans interaction de service, avec le vrai display ID et ses textures. Le document de camp n’est modifié qu’après Appliquer. Masquer l’aperçu ou fermer F8 supprime cette entité. F9 conserve la configuration et sa copie de secours.

Les six portails de capitale sont bien distincts dans les MPQ : 4393 Darnassus, 4394 Forgefer, 4395 Orgrimmar, 4396 Hurlevent, 4397 Pitons-du-Tonnerre, 4398 Fossoyeuse. Le problème précédent venait du `Portal::default()` identique à chaque ajout. L’ajout réutilise désormais le portail prévisualisé, ou alterne les capitales de la faction. Les anciens placements ne sont pas réécrits sans demande.

Validation ciblée : 82 tests de règles réussis, journal `D:\SurvivorsTools\tmp\menu-admin-rules.log`. La sonde native `WOW_SURVIVORS_VERIFY=menu_admin` exige un NOUVEAU profil avec le marqueur exact `isolated validation` et refuse toute sauvegarde préexistante. Elle actionne les composants Bevy réels via Pressed/Hovered et le gestionnaire GlueClicks : mage humain, annulation, changement avec conservation de l’or, confirmation de suppression, passage en préparation, aperçu sans mutation puis application de deux portails distincts. Lire le rapport STATUS et les captures ; des clics injectés ne prouvent pas le ciblage physique de la souris.

**État livré de cette révision :** compilation Windows terminée avec succès (`menu-admin-build-final.log`). Les 82 tests de règles ont été exécutés avant la demande d’arrêt des vérifications. À la demande du joueur, AUCUN essai natif de cette révision n’a été lancé : création, suppression, disposition visuelle et aperçu/application des modèles restent à tester par le joueur. Le dossier `D:\SurvivorsTools\menu-admin-proof\profile` contient seulement le marqueur de préparation, pas un rapport PASS. Ne pas considérer le code de sonde comme une preuve d’exécution.

L’EXE principal a reçu le build des corrections avant que le joueur ne lance le jeu. Après le commit 82abbe1, l’estampillage du launcher a réussi, mais la copie sur AzerothSurvivors.exe a été bloquée par le processus ouvert. L’EXE estampillé final est donc livré à côté : `out\AzerothSurvivors\AzerothSurvivors-82abbe1.exe`. Ne pas fermer le jeu du joueur automatiquement. Remplacer le nom principal seulement une fois le jeu fermé.

### Coffres et cadre de personnages — suite du 14 octobre

Le cadre de sélection utilise désormais six lignes compactes transparentes avec surbrillance dorée, titre et actions séparés par des marges. La capacité est partagée avec character_save par `menu::ROSTER_PAGE_SIZE`.

Ouvrir un coffre à portée, si l’or suffit, place `Run.pending_chest` et passe en `Phase::LevelUp`. Le même écran de trois cartes et les mêmes animations sont utilisés ; combats et temps de partie sont suspendus. Le titre devient COFFRE AU TRÉSOR et indique le prix. L’or n’est débité, le compteur augmenté et le coffre retiré qu’à la confirmation du choix. Les niveaux XP en attente restent intacts ; leurs cartes suivent avant de reprendre le combat.

`relic_rules.rs` décrit les objets temporaires propres aux sorts connus : grimoire (+40 % dégâts), rune (-35 % recharge), précision (+20 points de critique), focaliseur (+1 cible et +20 % dégâts, uniquement pour les sorts qui utilisent un nombre de cibles). Les effets sont appliqués aux casts réels et aux descriptions chiffrées. Ils fonctionnent indépendamment du rang maximum des sorts. Les reliques et le coffre en attente sont sauvegardés dans la Session via des champs serde(default) ; une nouvelle expédition les réinitialise. Le butin permanent des boss n’est pas modifié.

Référence demandée : `C:\Users\Nyaru\Downloads\AzerothSurvivors (1)\AzerothSurvivors\Data`. Lecture seule des deux MPQ : index et extrait de Spell.dbc dans `D:\SurvivorsTools\original-gameplay`. Spell.dbc contient 22357 lignes de 173 champs, avec les définitions des sorts Vanilla et les ressources d’interface. Aucun script de déroulement des coffres n’a été identifié dans ces archives ; ne pas prétendre que l’ensemble du gameplay de l’EXE original a été reproduit à partir du dossier Data.

À la demande du joueur, aucun test ni essai en jeu n’est exécuté pour cette révision. Compiler avec `scripts/build-windows.ps1 -ExecutableName AzerothSurvivors-coffres.exe` pour livrer à côté d’un jeu déjà ouvert. Le joueur effectue les essais.
