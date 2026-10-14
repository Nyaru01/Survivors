# Hubs de préparation Survivors

Les personnages Alliance arrivent dans le **Sanctum des Sorciers de Hurlevent** ; les personnages Horde dans la **Faille de l’Ombre d’Orgrimmar**. Les deux lieux réutilisent les données Vanilla 1.12.1 et proposent les mêmes services : marchand, artisan de fusion et portail de départ.

## Sources des lieux et des modèles

Les emplacements sont définis dans [`newhub_sites.rs`](../crates/benilla-app/src/survivors/newhub_sites.rs) à partir du dump SQL VMaNGOS `world_full_14_june_2021.sql` et des modèles référencés par les DBC. Les modèles des PNJ sont identifiés par leur `CreatureDisplayInfo.dbc`.

À Hurlevent, l’arrivée actuelle est la sortie SQL du Sanctum, en haut, aux coordonnées monde `(−9014.94, 873.326, 148.616)`. Les points des services viennent du groupe géométrique 284 dans `newhub_sites` et restent distincts du point d’arrivée. À Orgrimmar, l’arrivée s’appuie sur le spawn de Neeru (GUID 3415) ; les services sont ancrés au groupe WMO 138, avec des sols échantillonnés dans ses triangles de collision MOPY, à moins de 10 yards de l'arrivée.

Le portail actuel utilise l’affichage `GameObjectDisplayInfo.dbc` **4396**, rattaché au template GameObject `176296`, à l’échelle 1 : `World\\Generic\\ActiveDoodads\\SpellPortals\\MagePortal_Stormwind.m2`. Sa boucle dure 3334 ms et comporte quatre émetteurs avec leurs textures Vanilla. Les données sont chargées depuis la chaîne MPQ locale au runtime ; aucun asset du jeu n’est copié dans le dépôt. L’ancien affichage 1167 a été écarté après une capture montrant un portail géant dont le modèle mesure 31,7 yards de hauteur.

## Interaction et interface

### Extension de l’éditeur et personnages

F8 ouvre maintenant un panneau Blizzard d’aménagement : objets précédent/suivant, ajout/retrait de portails, nom éditable, destination, catalogue des modèles du client avec filtre et pagination. Le catalogue GameObjectDisplayInfo local comporte 32 entrées de portail ; les modèles PNJ proviennent de CreatureDisplayInfo, sans copier les assets. Les grands portails réduits gardent un volume de clic utilisable, et le retour à la pose originale conserve leur échelle.

Chaque faction conserve ses propres portails et modèles de services. Le fichier `survivors-hubs.txt` passe au format v2 ; le format v1 est migré en gardant les placements. Une copie `survivors-hubs.bak` précède l’écriture. Les destinations verrouillées restent verrouillées pour le personnage.

Le personnage peut sauter dans le camp. Les sacs, la fiche, les talents et le grimoire ne mettent plus les déplacements en pause. Les menus/options et la saisie de texte continuent de capturer les contrôles.

`character_roster.rs` et `character_save.rs` conservent plusieurs personnages dans la sauvegarde, chacun avec son équipement, or, talents, mondes et reprise. L’ancien fichier est migré en premier slot. Le menu est paginé et regroupe Préparation/Apparence/Reprendre/Options. Une reprise d’un autre personnage ne doit jamais être effacée par un `dead_t` résiduel au menu.

### Validation de l’extension du 14 octobre

Compilation Windows réussie, **80 tests de règles** réussis (`D:\SurvivorsTools\tmp\hub-expansion-rules.log`) et sonde native `STATUS=PASS` : `D:\SurvivorsTools\hub-expansion-proof-v2\profile\verification-hub.txt`. Cette sonde exerce le panneau Lua de l’éditeur (ajout, nom, destination, sauvegarde et retrait), puis les services et le portail des deux factions. Capture de l’éditeur : `shots\editor-named-portals.png`.

`loading-hd.png` dans ce profil a été relu : illustration embarquée, logo/barre du client et aucun nom de PNJ ni aide F8 au-dessus. Les PNG source sont 1672×941, affichés sur le viewport ; voir `assets/survivors/README.md` pour provenance et prompts. Le nouveau chargement conserve la readiness du moteur. La sauvegarde réelle est restée identique par SHA-256.

Les gestes physiques (saut, déplacement avec panneaux ouverts, navigation entre personnages) et chaque modèle du catalogue restent à essayer. Les tests de roster sont purs ; ne pas les présenter comme une validation souris/clavier de toute la sélection. Le premier essai `hub-expansion-proof` a échoué et ne constitue pas la preuve finale : PNG non décodé, API EditBox HasFocus absente en 1.12 et contrôle de transforms trop tôt après reconstruction, tous corrigés avant la sonde v2.

Le clic droit à portée ouvre le service ciblé ; `E` reste un raccourci complémentaire. À l’arrivée, aucun panneau marchand, fusion ou sac ne s’ouvre. Le portail est interactif dans le monde. Talents, Équipement, Grimoire et Sacs restent accessibles discrètement en bas de l’écran.

La fusion consomme trois objets blancs identiques — même emplacement et même rang — qui ne sont pas équipés, puis produit un objet vert permanent du rang supérieur (plafond 8), selon la progression déjà présente. L’identité du modèle d’objet Survivors est dérivée de l’emplacement, de la qualité et du rang ; chaque exemplaire conserve son identifiant d’instance.

## Validation

### Placement par le joueur

Dans le camp, **F8** ouvre l’outil de placement. **F6** choisit marchand, artisan ou portail ; marcher jusqu’au point voulu puis **F7** y pose le service avec l’orientation du personnage. **Page précédente / Page suivante** tournent l’objet de 15°, **Suppr** rétablit le placement original, **F9** sauvegarde. Fermer avec F8 puis s’éloigner du service pour l’utiliser. Les panneaux métier/options doivent être fermés. Les positions des deux factions sont conservées séparément dans `benilla-config/survivors-hubs.txt`, sans modifier les personnages. Les coordonnées enregistrées sont en repère Bevy, pas WoW. Les placements hors d’un rayon 3D de 90 yards de l’arrivée sont ignorés au chargement.

Le module `newhub_editor.rs` adapte les transforms des acteurs logiques : leurs modèles, volumes d’interaction et étiquettes suivent ensemble. `newhub_layout_rules.rs` valide le fichier (version, valeurs finies, indices et doublons). Cet outil de création appartient au mode Survivors et ne modifie aucune donnée Vanilla.

Validation de l’ajout : compilation Windows, 63 tests de règles réussis et sonde native des deux factions `STATUS=PASS` dans `D:\SurvivorsTools\hub-editor-proof\profile\verification-hub.txt`. L’aide F8 est visible dans la capture Alliance à 1620×690. Rapport des tests : `D:\SurvivorsTools\tmp\hub-editor-rules.log`. La sauvegarde réelle est inchangée. L’activation F8, le placement F7 et la sauvegarde F9 par gestes physiques restent à essayer ; la sonde native couvre le démarrage et les interactions habituelles, les tests de codec couvrent le fichier de positions.

Les 61 tests ciblés et trois vérifications mathématiques exactes passent. Les essais natifs des deux factions passent : sol, placement, cible projetée dans le cadre, portée, occlusion, ouverture par le gestionnaire commun, achat à 60 or, consommation des trois ingrédients, résultat vert de rang supérieur, portail vers le monde autorisé et déplacement en partie par événement clavier Bevy.

Les captures ont été relues à 1620 × 690 et 1920 × 1080, sans fenêtres métier à l’arrivée et avec les panneaux marchand/fusion ouverts séparément. La caméra initiale Alliance vise la salle afin d’éviter le portail décoratif existant. Les PNJ se trouvent autour du personnage ; tourner la caméra permet de les voir.

Preuves locales : `D:\SurvivorsTools\vanilla-hubs-delivery-proof\profile\verification-hub.txt` et `shots\` ; panneaux en petite résolution : `D:\SurvivorsTools\vanilla-hubs-final-small\profile\verification-hub.txt` et `shots\`. Les deux rapports portent `STATUS=PASS`. Le hash SHA-256 de la sauvegarde réelle est resté identique.

Limites : les tests utilisent des rayons projetés, le gestionnaire commun et des callbacks Lua, sans pilotage physique de la souris. Les clics/glissés réels, les coffres pendant un combat et la relance/reprise d’une partie existante restent à confirmer manuellement. Les coffres utilisent la même couche d’interaction et une protection contre deux ouvertures du même coffre dans une frame ; achats, sauvegarde et progression restent couverts par leurs tests de règles.

Les modèles des services utilisent les affichages CreatureDisplayInfo 191 et 243 : `Creature\HUMLMerchant\HUMLMerchant.m2` et `Creature\HUMLBlackSmith\HUMLBlackSmith.m2`, avec échelle et textures vérifiées dans la chaîne locale. L’affichage 19 a été rejeté car son échelle DBC est zéro.

Le portail Horde est placé sur le triangle de collision du groupe 138 à `(1794.876, -4382.030, -17.7444)`, à environ 6,7 yards de l’artisan. Son volume de clic suit la taille du portail compact. La recette affiche également les ingrédients partiels (1/3, 2/3) ; ils ne sont jamais consommés avant une recette complète valide.
