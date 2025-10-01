# Cleanup Crew

by Team3

## Team Members
* Advanced Topic Subteam 1: Procedural Generation 
	 * ryl50 : Ryan Liang
     * Wrw15 : William Waite
  	 * lml122 : Lucas Loepke
     * asg149 : Ansel Gunther
   

* Advanced Topic Subteam 2: Physics
	* vld18 : Vladimir Deianov
	* alv115 : Aidan Van't Hof
	* SAC496 : Sam Chung
	* dgg22 : Daniel Gornick
	

## Game Description

In the future, a massive space station suffers a disaster. You play as a scientist whose ship crashes into the station. Once, you were studying alien life, but now those creatures have escaped and turned hostile. The station is crawling with enemies, and your job is to clear it room by room until all the enemies are gone. Each room is a new level, with layouts, enemies, and obstacles that change every time you play. Some enemies rush you, others hide and hunt. This makes combat fast and dynamic. The environment panels can break and turn off gravity, air can leak into space forcing you into crisis. You can dash to dodge attacks (maybe), or even use explosions to launch yourself across the arena. The story is told through items you find, letting you uncover what really happened on the station as you fight to survive.

The game is going to represent a 2d Bird's eye dungeon crawler. Game is going to have melee and medium ranged enemy attack types. There will be special mob unit. Player will have one stat upgrade per room cleared. Game difficulty will be scale as you get deeper.

All the concept art will be hand drawn by team members. 


## Advanced Topic Description

### Procedural Generation
Levels are built as a sequence of procedurally generated rooms. Each room has a randomly generated size, enemy placement, and obstacles. Rewards differ from room to room. The starting state of the fluid dynamic simulation for each room is randomly generated using Perlin noise. Environmental specifications, such as broken panels, are going to be added into the generation system so hazards feel dynamic and tied to the theme of the space station.

Algorithms:
Binary Space Partition (BSP) Rooms
Perlin noise to seed fluid dynamics simulation
https://christianjmills.com/posts/procedural-map-generation-techniques-notes/#binary-space-partition


### Physics:
Environmental Physics will include fluid dynamics. This mechanic will be implemented by Particle-Based Methods algorithms and
The Lattice Boltzmann Method
https://medium.com/@ethan_38158/how-to-write-a-fluid-simulation-in-rust-lbm-1aaaee9c2a5a

## Midterm Goals
* A basic testing room is created for the player to move around in and test functions (procedural generation)
* Basic Map / object Sprites created
* Procedural Generation API should be agreed upon
* Fluid dynamics generation API should be agreed upon
* Basic liquid dynamics algorithm is researched/decided apon for gas simulation
* Player Movement works
* Player Sprites created
* Gun can shoot bullets/projectiles
* Enemy sprites // 1 attack animation
* Player/Enemy HP working
* Basic Enemy movement works
* Objects have collision (ex. tables)
* Enemy collision working // collision with player and with the environment


...

## Final Goals

* Simulation of fluid dynamics of gases within rooms of the space station (motion within room, and gasses leaving room) (10%)
* Player/enemies modeled in fluid dynamics simulation (5%)
* Objects modeled in fluid dynamics simulation (5%)
* Players and enemies take damage from objects hitting them due to fluid dynamics (5%)
* Player/enemies damaged by lack of air (5%)
* Player/enemies can damage space station and affect fluid dynamics simulation (5%)
* Fluid dynamics starting state randomly generated using Perlin noise(10%)
* Level layout and room layout are different everytime (5%)
* Rooms are structured and connected using BSP generation (10%)
* End level screen displayed (5%)
* Rewards drop for completing a room (5%)
* Player can move in 4 dimensions and shoot (5%)
* Have 2 basic enemy types (5%)

...

## Stretch Goals

* Add a second weapon type
* Add a Reaper mob that will chase the player if they spend too long clearing the level
