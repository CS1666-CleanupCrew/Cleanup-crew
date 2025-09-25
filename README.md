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

Levels are built as a sequence of procedurally generated rooms. Each room has a fixed size but changes in layout, enemy placement, and obstacles. Rewards differ from room to room. Environmental specifications such as broken panels, zero gravity are going to be added into the generation system so hazards feel dynamic and tied to the theme of the space station.
We are going to have total 6 differently sized rooms.

Algorithms:
Simple Room Placement 
Binary Space Partition (BSP) Rooms
https://christianjmills.com/posts/procedural-map-generation-techniques-notes/#binary-space-partition
### Physics: 


 Environmental Physics will include fluid dynamics. This mechanic will be implemented by Particle-Based Methods algorithms and
The Lattice Boltzmann Method
https://medium.com/@ethan_38158/how-to-write-a-fluid-simulation-in-rust-lbm-1aaaee9c2a5a

## Midterm Goals
* A basic testing room is created for the player to move around in and test functions (procedural generation) (15%) 
* Basic Map / object Sprites created (5%)
* Procedural Generation API should be agreed upon (10%) 
* Basic liquid dynamics algorithm is researched/decided apon for gas simulation (10%) 
* Player Movement works (5%) 
* Player Sprites created (5%) 
* Gun can shoot bullets/projectiles (15%) 
* Enemy sprites // 1 attack animation (5%) 
* Player HP working (5%) 
* Enemy HP working (5%) 
* Basic Enemy movement works (5%) 
* Objects have collision (ex. tables) (5%) 
* Enemy collision working // collision with player and with the environment (10%) 


...

## Final Goals

* Simulation of fluid dynamics of gases runs (10%)
* Simulation of fluid dynamics of gases are accurate and finished (5%)
* Simulation of fluid dynamics of gases interacts correctly with objects and are finished (2.5%)
* Simulation of fluid dynamics of gases interacts correctly with player and are finished (2.5%)
* Simulation of fluid dynamics of gases interacts correctly with enemies and are finished (5%)
* Calculation on the rate at which the gasses leave the room are accurate (die after a certain amount of time) (5%)
* Player/enemies can damage space station and cause the physics simulation to begin (10%)
* Rooms are structured and put in order with a logical continuation (10%)
* All rooms are completed (5%)
* Graceful ending to the levels, not an abrupt ending (5%)
* Level layout and room layout are different everytime (20%)
* Rewards drop for completing a room (10%)
* Players and enemies take damage from objects hitting them due to fluid dynamics (5%)
* Sound effects should be made. Potentially background music might be added. (5%)


...

## Stretch Goals

* Add a unique weapon
* Add a Reaper mob that will chase the player if they spend too long clearing the level
