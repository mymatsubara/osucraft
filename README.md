# osucraft

An implementation of the rhythm game [osu!](https://osu.ppy.sh/home) inside of minecraft using [valence](https://github.com/valence-rs/valence).

![gameplay](/assets/gameplay.gif)

> 🎬 Showcase video: [here](https://www.youtube.com/watch?v=Yw5VYiOWDWk)

Osucraft is a custom minecraft server implemented using the [valence](https://github.com/valence-rs/valence) framework, which I highly recommend cheking out if you're interested in building your own custom minecraft server using [rust](https://www.rust-lang.org/).

Sugestions are more than welcomed, so feel free to open an issues or pull requests with possible improvements.

> ⚠️ This isn't a really serious project, so the code is far from perfect and there are probably a ton of bugs.

# Running osucraft

### Windows

Download the executable [here](https://github.com/mymatsubara/osucraft/releases/tag/0.1.0) and run it. Now you should be to access the server on the address `127.0.0.1:25565`.

### Linux and MacOS

Building the project on your machine:

1. Clone this project
2. Run `cargo build --release` ([rustup](https://www.rust-lang.org/tools/install) required)
3. Run the executable `./target/release/osucraft`
4. You'll be running osucraft server on `localhost`

# Frequently asked questions

### How hitcircles are made?

Hitcircles rings are made of many invisible [armor stands](https://minecraft.fandom.com/wiki/Armor_Stand) equipped with a correctly rotated block in their head slot. Using some trigonometry the armor stands are positioned to make up the ring and since armor stands are entities they can overlap each other allowing smooth circles.

### Are sliders and spinners implemented?

Unfortunately they are not implemented. Sliders are replaced by hitcircles and spinners are ignored. I don't promise anything, but maybe in the future I'll try to implement them.

### Is any resource pack required?

They are not required, everything works using the default vanilla minecraft resource pack.

### How to improve my minecraft performance for osucraft?

My recommendation is to [allocate more RAM to minecraft](https://youtu.be/185lJ0M-58I) and to use [Fabric with Sodium, Lithium and Starlight](https://gist.github.com/HexedHero/aab340a84db51913cb1106c2d85f4e4f).
