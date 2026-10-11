# Brief

## The request (Sam, 11 October 2026)

> Please use the Builder repo as a base in the directory above, and use the work and repos of Sam gilson at sgilson7.github.io, which can all be found one directory up. please build out the MVP and deploy it to my website. you have approval for creating a public github repo, github pages, and approval to deploy to my website when its ready. any time you need to make 2D art and any kind of asset for the game, use the tikz latex prompt you can find in other repos, and use sam comm final as an inspiration image, alongside the menu screens of all of the sgilson7 games which use a similar build pipeline for images. now build the mvp for the game and deploy it, dont stop till its deployed, good luck. here is the prompt from my friend to follow to create the game containing all design decisions: the game should be 3d.

## The design (a friend's, verbatim)

> the game is a 4player, online multiplayer arc raiders like game, use the multiplayer from ribbit king for online 3d networking, and try and make it rollback if you can to improve latency. the gameplay itself is like arc raiders, but you are raiding old abandoned malls that have been filled with similar drones like arc raiders, that fly around in patterns, have weak spots and strong spots, and move like real life drones when shot and have behavior trees to control when they peek you, how they hover, how they circule you, and more advanced behaviors like trying to bait you out of cover. the gameplay for those playing is a 3rd person shooter. the main gimmick is that all gear you pick up during the run gets extracted like arc raiders, and you use what you find to build your guns, inlike a banjo kazooie nuts and bolts sort of way; im thinking its a little bit like building gear in gear master, with different base parts to your weapon, accessories, handles, and the damaging pieces really fire their shots out of the 3d rendered end of the gun. im imagining being able to build like a pistol with a scope and 4 double bareled shotgun barrels aligned in a square to make a 4x4 grid of barrels, or like an ak47 base with a rocket longer tip that fires rockets filled with slowing goo that corrodes armor but doesnt deal actual damage to enemies. the game is 4 player co-op, where you try to clear the mall or do some objective in it like find an old rare kid cudi shirt in an old hot topic, essentialy your characters are like hired mercernaries that go delving in these old dangerous drone filled malls with orders from clients, so each mission is a different order, all of it in the same map. build out the mvp with one map based ont eh soutpoint mall in durham nc, inckuding the outside streets area. you should start the mission in the back of your van like phantasmaphobia, the ghost hunting game, and when you leave the fan you are far in the parking lot and can see the mall in the distance, and the mall has several entraces to the outside so different missions start you in fron of different ones. there are minimal drones outside but there are some, but the majority of them are prowling in the mall. create 4 drone types that fly, and 2 that walk like spiders to start. there should be 5 different misions to do, and 5 of each gun making piece / type

## Done means

A 3D browser game, deployed on GitHub Pages and listed on sgilson7.github.io, built as a Builder project whose `./scripts/check.sh` passes, in which:

- one to four raiders play co-op, alone or online in their own browsers, meeting by room code or pasted invitation (Ribbit King's transport), with rollback netcode;
- a run starts in the back of the van far out in the lot, and each of five client orders parks the van in front of a different one of five entrances to one map of the Streets at Southpoint and its lots and Main Street;
- four flying drone kinds and two spider walkers, each with weak and strong spots, drone-like reactions to hits, and a behaviour tree (patrols, hovering, circling, peeking, baiting out of cover);
- guns are built from six kinds of part, five of each; barrels fire from their own rendered muzzles; what a raider carries back to the van goes to their stash, and what they took in is lost if they do not come back;
- every 2D picture is TikZ.

## Autonomy

Sam approved in advance: a public GitHub repository, GitHub Pages deploys from it, and deploying the website listing when the game is ready.

## Decisions left with Sam

- The name. "Raiders of the Lost Mart" is a working title.
- Whether the friend should be credited by name (the page says "a friend's design").
- Real names in the game: the mall is named (Southpoint), as are Hot Topic and Kid Cudi, because the design names them; every other store is generic. No logo or brand art is drawn.
