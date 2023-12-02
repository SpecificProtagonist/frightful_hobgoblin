# WIP!

This is a collection of code that may one day become a [GDMC](https://gendesignmc.wikidot.com/start) generator.

The aim of GDMC is to take an existing Minecraft map and generate a settlement within it, aiming at adaptability, functionality, evocative narrative and aesthetics. While most generators generate a static instance of a village, this once runs a simulation of the village getting constructed and replays it in Minecraft. Running the replay only requires Minecraft and no mods or external programs. It also aims to be much faster by working with the world directly instead of over an http interface and by using a fast language instead of a interpreted, highly dynamic one.

This code wasn't made with the intent to be useful for anyone else. There is little separation between framework and generator, no documentation/no comments, no focus on maintainability. Most importantly though, the internal representation of blocks only covers what I've needed myself.

The simulation works via an ECS. This means that objects such as villagers or trees are composed of components such as `Position` or `Tree`, which only carry data, and are queried by systems which implement behavior. Blocks are stored in raster format.

For buildings to be constructed, villagers need to transport goods to the construction site and then place the blocks. Goods are visible both in storage and in transport. Overall it aims to achieve a *Wuselfaktor* similar to Settlers 3/4.

Each simulation tick corresponds to one game tick. Each tick, the changes to the world get written out as Minecraft commands to run during replay. As this results in hundreds of thousands of commands, getting Minecraft to run them is tricky: Placing them in mcfunction files crashes MC even if they are never executed, as they are eagerly parsed. Instead they are stored in command storage (in nbt), which get loaded via the `data` command and executed via macros. Replays can be paused or fast-forwarded via a command.

The simulation is pseudorandom but deterministic (useful for debugging).

Performance-wise I haven't made many optimizations yet, but it world loading is parallelized and nbt encoding/gzip compression is offloaded to worker threads.
