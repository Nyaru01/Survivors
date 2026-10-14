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

Ajout après l’instantané initial : outil de placement du camp dans `newhub_editor.rs` et codec `newhub_layout_rules.rs`. F8 active, F6 choisit, F7 pose aux pieds, Page précédente/suivante tourne, Suppr remet l’origine et F9 écrit `benilla-config/survivors-hubs.txt`. Les overrides sont propres à chaque faction et séparés des sauvegardes de personnage. Voir `CONTROLS.md` et `docs/SURVIVORS-HUBS.md` ; le commit de référence en tête de ce document décrit la livraison précédente.

1. Clic droit réel sur PNJ/portail, glissés de caméra et priorité de l’UI ; déplacement dans les deux camps.
2. Coffres pendant un combat : clic, coût unique, équipement temporaire et séparation des objets permanents des boss.
3. Quitter puis reprendre une partie existante, conservation du personnage/talents/sacs et parcours mort → sélection → préparation.
4. Examiner le portrait marchand offline si son cercle reste noir ; aucune correction de ce point n’a été certifiée dans la livraison des hubs.

Les sondes envoient des interactions projetées, événements Bevy et callbacks Lua ; elles ne pilotent pas la souris physique. Ne pas annoncer ces parcours manuels comme testés. Les PNJ peuvent être masqués selon l’angle et les décors : ne pas promettre qu’ils sont tous visibles au premier cadrage ni une fidélité parfaite au client Blizzard.

Pour une reprise efficace : commencer par ces essais, traiter les défauts observés, conserver les données personnelles, puis relancer uniquement les validations concernées. Un changement documentaire seul ne nécessite pas de reconstruire l’EXE.
