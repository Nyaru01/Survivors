# Hubs de préparation Survivors

Les personnages Alliance arrivent dans le **Sanctum des Sorciers de Hurlevent** ; les personnages Horde dans la **Faille de l’Ombre d’Orgrimmar**. Les deux lieux réutilisent les données Vanilla 1.12.1 et proposent les mêmes services : marchand, artisan de fusion et portail de départ.

## Sources des lieux et des modèles

Les emplacements sont définis dans [`newhub_sites.rs`](../crates/benilla-app/src/survivors/newhub_sites.rs) à partir du dump SQL VMaNGOS `world_full_14_june_2021.sql` et des modèles référencés par les DBC. Les modèles des PNJ sont identifiés par leur `CreatureDisplayInfo.dbc`.

À Hurlevent, l’arrivée actuelle est la sortie SQL du Sanctum, en haut, aux coordonnées monde `(−9014.94, 873.326, 148.616)`. Les points des services viennent du groupe géométrique 284 dans `newhub_sites` et restent distincts du point d’arrivée. À Orgrimmar, l’arrivée s’appuie sur le spawn de Neeru (GUID 3415) ; les services sont ancrés au groupe WMO 138, avec des sols échantillonnés dans ses triangles de collision MOPY, à moins de 10 yards de l'arrivée.

Le portail actuel utilise l’affichage `GameObjectDisplayInfo.dbc` **4396**, rattaché au template GameObject `176296`, à l’échelle 1 : `World\\Generic\\ActiveDoodads\\SpellPortals\\MagePortal_Stormwind.m2`. Sa boucle dure 3334 ms et comporte quatre émetteurs avec leurs textures Vanilla. Les données sont chargées depuis la chaîne MPQ locale au runtime ; aucun asset du jeu n’est copié dans le dépôt. L’ancien affichage 1167 a été écarté après une capture montrant un portail géant dont le modèle mesure 31,7 yards de hauteur.

## Interaction et interface

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
