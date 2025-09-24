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

In the future, a massive space station suffers a disaster. You play as a scientist whose ship crashes into the station. Once, you were studying alien life, but now those creatures have escaped and turned hostile. The station is crawling with enemies, and your job is to clear it room by room until you reach the final boss in the most important chamber.Each room is a new level, with layouts, enemies, and obstacles that change every time you play. Some enemies rush you, others hide and hunt, and bosses use smart tactics like cover and movement to make battles more intense. This makes combat is fast and dynamic. Bullets can bounce, weapons feel powerful, and the environment panels can break and turn off gravity, air can leak into space, and gas can spread through the station. You can swing on ropes, dash to dodge attacks, or even use explosions to launch yourself across the arena.The story is told through items you find, letting you uncover what really happened on the station as you fight to survive.

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


 Environmental Physics will include fluid dynamics. This mechanic will be implemented by Particle-Based Methods algorithms


## Midterm Goals
* Player can move around the room and the camera setup works
* A layout of one room should be created and a player should be able to navigate around it
*  Procedural Generation API should be agreed upon
* Basic enemy movement with 1 attack animation

...

## Final Goals

* 35%: Rooms should be structured and put in order. Levels should have a logical continuation. 
* 35%: All rooms should be finished and put in order. The game should be gracefully concluded.
* 10%: Sound effects should be made. Potentially background music might be added

...

## Stretch Goals

* Add a unique weapon
* Add a new interactive object
