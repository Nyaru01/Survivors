# Controls

**Azeroth Survivors** — top-down, one hand on WASD, the other on the wheel.

| When | Key / Input | Action |
|---|---|---|
| Menu | Click a race, gender, class and zone | make your pick (**Randomize** rolls one) |
| Menu | **Enter** (or **Enter World**) | start the run; the world travels to the zone picked |
| Playing | **W / S / A / D** or **arrow keys** | move the hero (locked camera: the hero faces the cursor, backpedals and strafes) |
| Playing | **Space** | jump |
| Playing | **F** (from level 5) | your class's movement spell: Blink, Charge… aim at the cursor |
| Playing | **C** | unlock / lock the camera |
| Any | **G** | toggle the picture polish (`?grade=0` starts it off) |
| Playing | **mouse wheel** | zoom, from over the shoulder out to 70 yd |
| Playing | **right mouse button + drag** | unlock, swing and tilt the camera directly |
| Playing | **left mouse button + drag** (camera unlocked) | swing and tilt the camera |
| Playing | **Esc** | pause |
| Menu / paused / playing | **F10** or **Options** | graphics, controls and sound preferences |
| Playing | **E** or the chest button, within 3 yd | open a treasure chest: 40 gold, then 20 more per chest |
| Playing / paused / menu | **P** or **Boutique** | buy health potions and inspect equipment |
| Playing / paused | **H** or **Boire une potion** | restore 50% health; 20-second cooldown |
| Playing | — | learned abilities cast automatically by default |
| Playing | **1 / 2 / 3 / 4 / 5 / 6** or click a learned spell | request that spell, respecting its cooldown |
| Playing | **right-click a learned spell** | toggle its auto-cast; a green edge means automatic |
| Level up | **1 / 2 / 3** (or click a card) | pick an upgrade |
| Paused | **Esc** | resume |
| Game over | **Enter** (or **Play again**) | back to the menu |
| Any | minimap **+ / −** / minimize | minimap zoom / collapse |

The camera follows the hero, locked top-down by default (laptop-friendly). A right-drag unlocks
and turns it directly; **C** locks it back into the top-down view. The spell bar is at the bottom
centre, with Blizzard's original stone frame, griffons and experience strip, each class's
learned spells and their cooldowns. Spells remain automatic by default; click or use 1–6
to request one, or right-click its icon to toggle automatic casting. Player and enemy
portraits and health are at the top left. In the menu it slowly orbits the
preview hero. Your race, gender, class and zone are remembered across reloads.

Create your character at the menu: click the name field to enter a nickname (Enter to confirm),
then adjust skin, face, hair style, hair colour and facial hair. The preview shows the changes.
**Nouvelle partie** starts a new adventure; **Reprendre la partie** restores the suspended
adventure in pause, including enemies, living bosses, pickups and treasure chests.

Gold and experience both drop on the ground and fly toward you within pickup range. Gold,
potions and four equipment slots remain with your character between adventures. Chests equip
upgrades automatically: class weapons, chest armour, rings and boots grant actual stat bonuses.
These equipment upgrades currently affect stats and the inventory; their appearance is not
yet substituted on the character model. Potions cost 20 gold, with three purchases per adventure
and a carrying limit of three. Using one at full health does not consume it.

A boss arrives every three minutes independently of earlier living bosses. The next arrival
is displayed above the world; pause and level-up screens freeze the countdown. After the
15-minute survival goal, remaining bosses must be defeated before victory.

Your character and current adventure are saved every two seconds and when the window closes,
in **benilla-config/survivors-save.toml** beside the executable, with a **.bak** recovery copy.
Keep this folder when updating the executable. Preferences are stored separately in
**survivors-options.txt**. The supplied French 1.12.1 installer is kept read-only; it is not
used as an already-installed runtime Data folder.
