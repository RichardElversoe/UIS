# What is UIS?
  UIS (Universal Interface Script) is a simple, declarative scripting language designed specifically for building user interfaces. It is designed to reduce frontend complexity by offering a unified, lightweight, and readable structure — built entirely around reusable, self- documenting components.
  
  At its core, UIS revolves around three pillars:
    🧱 Components — Define the structure of your application, and can be nested, reused, and modified.
    🎛️ Properties — Define data or styling attached to a component. These describe what the component looks and feels like.
    🔀 Conditions — Define behavioral logic for a component, based on its properties or context.
  
  In addition to this, UIS is made up of three main parts:
    UIS — the language itself.
    UIS-Engine — an interpreter that reads and processes UIS files.
    UIS-Renderer — a lightweight renderer that turns UIS data into on-screen visuals.
  
  💡 Note: UIS is still highly experimental and evolving. The syntax is mostly stable, but the engine and renderer are in active development.

# Why UIS?
  UIS was created from the ground up to simplify frontend development while enabling powerful, modular, and clean UI logic. Here's why it 
  exists:
  
  💡 Simple & expressive syntax:
  UIS is designed to be easy for both developers and creatives to learn — it’s readable like HTML, expressive like CSS, and functional like 
  JavaScript, but unified into a single format.
  
  🧩 Modular and reusable by default:
  UIS encourages building small, isolated components that can be reused, customized, and nested — leading to cleaner, maintainable code with 
  less repetition.
  
  🗂️ Self-documenting structure:
  Thanks to its readable syntax and strict component-property-condition model, UIS code naturally documents itself. You can mostly 
  understand what a file does just by reading it.
  
  🔁 Clear frontend–backend separation:
  UIS makes collaboration smoother by clearly dividing UI/UX responsibilities from backend logic. Frontend developers can focus on the 
  interface, while backend developers handle logic — with minimal friction.
  
  🐍 Standalone & embeddable:
  UIS can run like a scripting language (e.g., Python), or be embedded directly into Rust applications, making it suitable for everything 
  from web UIs to game interfaces.
  
  ⚡ Lightweight and fast:
  UIS avoids the weight of traditional frontend stacks. It aims to be CPU- and RAM-efficient, making it ideal for performance-sensitive 
  environments.
  
  🎯 Purpose-built for UI — and nothing else:
  UIS isn't trying to replace general-purpose languages. It's built to do one thing extremely well: define and control user interfaces.

# License
  This project is licensed under the Mozilla Public License 2.0 (MPL-2.0), with an additional Naming and Attribution Clause.
  
  What this means for you:
  ✅ You are free to use, modify, fork, and distribute this project.
  ✅ You can build commercial or closed-source software using UIS, UIS-Engine, or UIS-Render.
  ✅ You do not have to open-source your own apps, games, or tools that use UIS.
  📌 You must include a small credit somewhere visible (e.g., in an "About" screen or documentation).
  
  If you modify or fork this project:
  🖊️ Please keep attribution to the original authors in your version.
  🔁 You’re encouraged to keep changes compatible with the main branch (so improvements can flow back upstream).

# Contributions
  We welcome and appreciate all contributions to UIS — whether it's code, documentation, design, testing, or suggestions.

  🧪 Active Development
    - Development currently happens on the v0.0.1 branch.
    - All pull requests should target this branch unless otherwise stated.
  
  🌿 Branching Model
    - main holds the latest stable version (only updated on releases).
    - v0.0.1 is the current active development branch.
    - Use feature branches for contributions:
        
    feature/your-topic
    fix/your-bug
    doc/your-improvement
  
  🛠️ How to Contribute
  
    # 1. Fork and clone the repo
    git clone https://github.com/RichardElversoe/UIS.git
    cd UIS
    
    # 2. Create a feature branch
    git checkout -b feature/your-contribution
    
    # 3. Make your changes
    cargo fmt
    cargo check
    
    # 4. Commit and push
    git commit -m "Add: helpful description"
    git push origin feature/your-contribution
  
  🧭 Contribution Areas
    We're looking for help with:
      🧑‍💻 Core development (parser, runtime, error handling, etc.)
      🪧 Logo / visual identity
      📘 Documentation and guides
      🛠️ Tooling and editor support
      🧾 A proper “How to write UIS” tutorial
      🐞 Filing issues and helping triage
    💬 Not sure where to start?
      check the Issues or open a new one — ideas, bugs, and questions are all welcome!
    
# Thank You ❤️
  All contributions are appreciated — small or big.
  Help shape the future of UIS and make it something truly useful and accessible to others.
