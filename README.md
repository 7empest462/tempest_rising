# 🌌 Tempest Rising

[![Rust](https://img.shields.io/badge/Language-Rust_2024-orange.svg)](https://www.rust-lang.org/)
[![Engine](https://img.shields.io/badge/Engine-Bevy_0.19-blue.svg)](https://bevyengine.org/)
[![Physics](https://img.shields.io/badge/Physics-Avian3D_0.7-green.svg)](https://github.com/JRefent/avian)
[![GUI](https://img.shields.io/badge/GUI-bevy__egui_0.41-purple.svg)](https://github.com/mvlabat/bevy_egui)

**Tempest Rising** is an open-world 3D sci-fi survival, exploration, and building action game built with **Rust** and the **Bevy Engine**. Explore an alien planet featuring binary suns, black hole gravitational lensing, deep subterranean caves, procedural ragdoll physics, dynamic water simulation, customizable armory, resource harvesting, building construction, companion animal domestication, and starship restoration.

---

## ✨ Features & Highlights

### 🦊 Fox Domestication & Companion Defenders
* **Individual Animal Friendship**: Approach wild foxes and offer treats (`T` key) to build friendship step-by-step (1/3 ❤️, 2/3 ❤️, 3/3 ❤️).
* **Named Loyal Pets**: Domesticated foxes receive unique individual names (*Sparky, Ember, Jasper, Rusty, Pippin...*) and wear glowing golden companion collars!
* **Active Combat Support**: Your tamed companion foxes follow you faithfully and leap into battle to pounce on hostile creatures (*Monsters, Triangaroos, Polypugs*) to defend you!

### 🚀 Exploration & Physics Engine
* **Seamless Camera Views**: Switch dynamically between **Third-Person**, **First-Person**, and **Orbit/Spectator** camera modes (`V` key).
* **Verlet Ragdoll Physics**: Collapse into floppy ragdoll physics anytime (`G` key) or tumble down steep mountain cliffs with dynamic recovery.
* **GPU Water Compute & Swimming**: Buoyant swimming system with turbo sprint, deep diving (`C` / `Ctrl`), surfacing, and real-time splash dynamics.
* **Underground Cave System**: Descend into subterranean caverns with ladders, mine bioluminescent crystal nodes, and discover Ancient Coffers.

### 🌌 Cosmic Skybox & Celestial Bodies
* **Binary Suns**: Golden Sun and Cyan Sun illuminating twilight alien horizons.
* **Gravitational Lensing Black Hole**: Event horizon surrounded by swirling relativistic particle accretion disks and Einstein ring halos.
* **Planetary Ring System**: Massive Saturn-style planetary rings spanning across the sky.

### ⚔️ Combat & Procedural Armory
* **3D Double-Sided Battleaxe**: Crafted melee tool with dual crescent blades, socket collar, and beveled cutting edges for chopping trees and rocks.
* **Firearms Roster**: Pistol, Heavy Revolver, Assault Rifle, and High-Velocity Sniper Rifle featuring procedural two-handed holding stances and recoil.
* **Deployable Robot Trilobite Defenders**: Summon automated companion combat drones (`X` key) to hunt hostiles and protect your perimeter.

### 🛠️ Crafting, Building & Quest System
* **Resource Harvesting**: Chop timber, mine stone, copper, iron, silver, gold, platinum, steel, granite, and crystal shards. Collect creature pelts and alien tech.
* **Modular Building Placement**: Construct brick walls, timber palisades, cyber metal walls, watchtowers, staircases, and ramps (`B` key building mode).
* **High-Tech Cyber Flight Suit**: Craft advanced flight armor to unlock 3D flight controls (`F` key) and high damage resistance.
* **Crashed Starship Restoration**: Collect key repair subsystems to restore your downed starship and pilot it into orbit.
* **Alien Barter Station**: Trade rare gold and harvested materials with alien merchants for high-tech components.

### 💾 Progress Saving & Customization
* **Persistent Progress System**: Quick Save (`F5`) and Quick Load (`F9`) hotkeys + HUD buttons saving inventory, tamed companions, location, equipment, ammo, and character outfits to `save_game.json`.
* **3D Character Designer**: Customize gender, height, weight, hair styles, colors, and outfit styles (*Sci-Fi Suit, Tactical Armor, Stylized Hero, Skeleton Exo-Frame, Classic Mannequin*).
* **Integrated Editors**: Includes a Map Editor and Built-in Sprite Editor for texture manipulation.

---

## 🎮 Controls Summary

| Key / Input | Action |
| :--- | :--- |
| **W, A, S, D** | Move / Strafe |
| **Shift** | Run / Turbo Swim Sprint |
| **Space** | Jump / Swim Up / Climb Ladder |
| **Ctrl / C** | Crouch / Dive Down (Water) |
| **Mouse Look** | Aim Crosshair & Camera Rotation |
| **Left Click** | Shoot Firearm / Swing Melee Weapon |
| **1 ..= 5** | Switch Weapon Slot (*Melee, Pistol, Revolver, Rifle, Sniper*) |
| **R** | Reload Current Firearm |
| **V** | Toggle View Mode (*Third-Person, First-Person, Orbit*) |
| **F5** | **Quick Save Progress** |
| **F9** | **Quick Load Progress** |
| **T** | **Offer Treats to Wild Fox (Build Friendship / Tame Companion)** |
| **F** | **Toggle High-Tech Flight Mode (When Cyber Suit Equipped)** |
| **X** | **Deploy / Dismantle Robot Trilobite Defender** |
| **H** | Toggle Tactical Headlamp |
| **Q** | Use Health Pack (+35 HP) |
| **B** | Toggle Modular Building Placement Mode |
| **G** | Collapse into Ragdoll Physics |

---

## ⚙️ Building & Running Locally

### Prerequisites
* [Rust](https://www.rust-lang.org/tools/install) (2024 edition or newer)
* Graphics drivers with Vulkan / Metal support

### Running Development Build
```bash
# Clone repository
git clone git@github.com:7empest462/tempest_rising.git
cd tempest_rising

# Run dev profile
cargo run
```

### Building Optimized Release
```bash
cargo build --release
```

---

## 📄 License
Licensed under the [MIT License](LICENSE).
