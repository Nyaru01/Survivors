# Commandes — Azeroth Survivors

Le jeu utilise les données et les fenêtres Blizzard du client français 1.12.1.

| Commande | Action |
|---|---|
| Personnaliser le personnage | Ouvrir la création Blizzard : race, classe, sexe, apparence et pseudo |
| Nouvelle partie / Entrée | Commencer dans la zone choisie |
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
| P | Ouvrir le marchand de potions |
| H ou clic droit sur la potion du sac | Boire une potion de soins |
| E près d'un coffre | Acheter son contenu : 40 pièces d'or, puis 20 supplémentaires par coffre |
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

Un boss apparaît toutes les trois minutes, même si les précédents sont vivants.
La pause et le choix d'amélioration suspendent le temps de la partie.

La sauvegarde automatique conserve le personnage, l'or, les potions, l'équipement
et la partie en cours, y compris ennemis, boss, objets au sol et recharges.
Conservez le dossier `benilla-config`, en particulier `survivors-save.toml` et `.bak`.
Les options et raccourcis Blizzard sont enregistrés par le moteur dans ce même dossier.

`Data-frFR` est prioritaire lorsqu'il existe à côté de l'EXE ; `Data` reste le repli.
Le client WoW fourni est conservé ; sa copie de travail a été patchée sur D:.
