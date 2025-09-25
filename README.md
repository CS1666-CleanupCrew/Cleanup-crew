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

In the future, a massive space station suffers a disaster. You play as a scientist whose ship crashes into the station. Once, you were studying alien life, but now those creatures have escaped and turned hostile. The station is crawling with enemies, and your job is to clear it room by room until you reach the final boss in the most important chamber.Each room is a new level, with layouts, enemies, and obstacles that change every time you play. Some enemies rush you, others hide and hunt, and bosses use smart tactics like cover and movement to make battles more intense. This makes combat is fast and dynamic. Bullets can bounce, weapons feel powerful, and the environment panels can break and turn off gravity, air can leak into space, and gas can spread through the station. You can swing on ropes, dash to dodge attacks, or even use explosions to launch yourself across the arena.The story is told through items you find, letting you uncover what really happened on the station as you fight to survive. Number of mobs are dependent on the room size. 

The game is going to represent a 2d Bird's eye dungeon crawler. Game is going to have Melee and  medium range enemy attack types. There will be special mob units. Player will have one weapon upgrade per room cleared. Game difficulty will be increasing  as you get deeper.

All the concept art will be hand drawn by team members. 


## Advanced Topic Description

### Procedural Generation

Levels are built as a sequence of procedurally generated rooms. Each room has a fixed size but changes in layout, enemy placement, and obstacles. Rewards differ from room to room. Boss's locations in rooms are also procedurally generated but designed. All of the Bosses are of the same level. Environmental specifications such as broken panels, zero gravity are going to be added into the generation system so hazards feel dynamic and tied to the theme of the space station.
We are going to have total 6 rooms.

Algorithms:
Simple Room Placement 
Binary Space Partition (BSP) Rooms
https://christianjmills.com/posts/procedural-map-generation-techniques-notes/#binary-space-partition
### Physics: 


 Environmental Physics will include fluid dynamics. This mechanic will be implemented by Particle-Based Methods algorithms and
The Lattice Boltzmann Method
https://medium.com/@ethan_38158/how-to-write-a-fluid-simulation-in-rust-lbm-1aaaee9c2a5a

## Midterm Goals
* A basic testing room is created for the player to move around in and test functions  (procedural generation)
* Basic Map / object Sprites created
* Procedural Generation API should be agreed upon
* Player Movement works
* Player Sprites created
* Gun can shoot bullets/projectiles
* Enemy sprites // 1 attack animation
* Player HP working
* Enemy HP working
* Basic Enemy movement works
* Objects have collision (ex. tables)
* Enemy collision working // collision with player and with the environment


...

## Final Goals

*16.7%: Rooms should be structured and put in order. Levels should have a logical continuation.
*16.7%: All rooms should be finished and put in order. The game should be gracefully concluded.
*16.7%: Sound effects should be made. Potentially background music might be added.
*16.7%: Fluid dynamics of all gases should be finished.
*16.5%: Level layout and room layout is different every time.
*16.7%: Rewards for completing a room drop.

...

## Stretch Goals

* Add a unique weapon
* Add a new interactive object
