# Commandes — Azeroth Survivors

Le jeu utilise les données et les fenêtres Blizzard du client français 1.12.1.

| Commande | Action |
|---|---|
| Personnaliser le personnage | Ouvrir la création Blizzard : race, classe, sexe, apparence et pseudo |
| Préparer le personnage / Entrée | Ouvrir la préparation : talents, équipement et grimoire |
| Clic droit ou E près du portail | Commencer le monde sélectionné après la préparation |
| Reprendre la partie | Retrouver la partie sauvegardée, initialement en pause |
| Touches de déplacement / flèches | Déplacer le personnage ; raccourcis modifiables dans le menu Blizzard |
| Espace | Sauter |
| Clic droit + glisser | Tourner et incliner la caméra |
| Molette | Zoomer |
| Ctrl+C | Revenir à la vue du dessus |
| Échap / F10 / Options | Ouvrir le menu Blizzard : graphismes, son et raccourcis |
| C | Ouvrir la fiche du personnage et son équipement |
| B | Ouvrir le sac à dos |
| N / bouton des talents | Ouvrir les trois arbres Blizzard de la classe |
| P | Ouvrir le grimoire des sorts de la classe |
| J en partie | Ouvrir le marchand de potions ; au hub, interagir avec le PNJ |
| H ou clic droit sur la potion du sac | Boire une potion de soins |
| Clic droit ou E près d'un coffre | Acheter son contenu : 40 pièces d'or, puis 20 supplémentaires par coffre |
| 1 à 6 ou clic sur un sort | Déclencher un sort appris, si sa recharge est terminée |
| Clic droit sur un sort | Activer ou désactiver son lancement automatique |
| F, à partir du niveau 5 | Utiliser le sort de déplacement de la classe |
| 1 / 2 / 3 lors du changement de niveau | Choisir une amélioration |

Les sorts se déclenchent automatiquement par défaut et restent propres à la classe.
Les déclenchements manuels partagent leur recharge avec les déclenchements automatiques.

Les talents sont conservés par classe entre les aventures. Chaque nouveau niveau maximal
atteint du niveau 10 au niveau 60 apporte un point, soit 51 au total ; recommencer une
partie ne redonne pas ces points. Rangs, paliers et prérequis sont vérifiés avant la dépense.
Les descriptions signalent les talents indisponibles dans ce mode : aucun point ne peut
y être dépensé. Les adaptations au combat automatique sont indiquées dans l’infobulle.
Les capacités débloquées par un talent rejoignent les choix de sorts lors des changements
de niveau. Les capacités déjà acquises dans une ancienne sauvegarde restent utilisables.

Les potions coûtent 20 pièces d'or. Le stock est limité à trois achats par aventure,
et le personnage peut en porter trois. Une potion rend 50 % de la vie maximale,
avec une recharge de 20 secondes. À vie pleine, elle n'est pas consommée.

Les coffres équipent automatiquement une amélioration d'arme, de torse, d'anneau
ou de bottes. Les bonus affectent les caractéristiques. Le remplacement visuel
des pièces sur le modèle du personnage reste à réaliser.

Un monde comporte quatre boss, apparaissant à 3, 6, 9 et 12 minutes, même si les précédents
sont vivants. Vaincre les quatre en restant vivant termine le monde et débloque le suivant.
Le premier monde dépend de la race : Elwynn pour les humains, Durotar pour les orcs/trolls,
Dun Morogh pour les nains/gnomes, Teldrassil pour les elfes, Mulgore pour les taurens,
Clairières de Tirisfal pour les morts-vivants. Un humain débloque ensuite la Marche de l’Ouest.
Les déblocages sont sauvegardés. Les anciennes sauvegardes conservent leur zone précédente.
Après la mort, le bouton « Sélection du personnage » ou Entrée revient au choix du personnage.
La caméra reste fixe pendant la sélection. Au hub et en partie, clic droit + glisser pour la tourner et molette pour zoomer.
La pause et le choix d'amélioration suspendent le temps de la partie.

Dans la fiche d’équipement, cliquer sur une pièce la retire vers le sac ; un clic droit sur la
pièce du sac la remet. L’objet reste possédé : seuls ses bonus actifs changent.

La sauvegarde automatique conserve le personnage, l'or, les potions, l'équipement
et la partie en cours, y compris ennemis, boss, objets au sol et recharges.
Conservez le dossier `benilla-config`, en particulier `survivors-save.toml` et `.bak`.
Les options et raccourcis Blizzard sont enregistrés par le moteur dans ce même dossier.

`Data-frFR` est prioritaire lorsqu'il existe à côté de l'EXE ; `Data` reste le repli.
Le client WoW fourni est conservé ; sa copie de travail a été patchée sur D:.

Camp 3D : déplacement ZQSD/WASD selon les options, ou flèches.
Approchez un PNJ et faites un clic droit : marchand ou artisan. E interagit avec le service proche.
Clic droit ou E près du portail : entrer dans le monde sélectionné.
Les menus de talents, d’équipement et de sacs restent accessibles au camp.
Coffres (E) : bonus temporaires, conservés à la reprise de la même expédition.
Boss : butin permanent dans les sacs ; clic droit pour équiper une pièce.
Artisan : 3 pièces blanches identiques non équipées donnent une pièce verte de rang supérieur.

Placement du camp : F8 ouvre/ferme l’outil. F6 choisit Marchand, Artisan ou Portail.
Marchez à l’emplacement voulu puis F7 pose le service à vos pieds.
Page précédente / Page suivante tournent le service de 15°. Suppr rétablit sa position d’origine.
F9 sauvegarde les positions pour les prochaines sessions (Alliance et Horde séparées).
Le fichier benilla-config/survivors-hubs.txt est indépendant de la sauvegarde du personnage.
F8 ouvre aussi le catalogue des modèles PNJ/portails du client : filtre de recherche et pages.
Ajouter un portail crée un passage indépendant ; renommer et choisir son monde dans l’éditeur.
Les mondes verrouillés restent inaccessibles. Taille +/− ajuste le portail ; F9 conserve tout.
Le format précédent de placement est repris automatiquement ; survivors-hubs.bak garde la copie précédente.
Sélection : plusieurs personnages, bouton Créer un personnage puis clic sur son nom pour le renommer.
Chaque personnage conserve ses propres sacs, talents, or et partie en cours. Navigation Précédents/Suivants.
Préparation : Espace saute ; sacs, fiche et talents peuvent rester ouverts pendant le déplacement.
Fermez les panneaux métier/options avant d’utiliser l’outil. Les services sont suspendus pendant le placement.
