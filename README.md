Read the [Proof of Concept](./PoC.md) (a great name) to basically do it yourself currently.

# Motivation

Development in NixOS is _painful_, compared to other distributions to say the least. While the prospect of reproducibility is well-received to an end-user using a "finished" product, for a university student like me, such fixed development instead becomes a liability for productivity.

There are a lot of examples that can be explicitly stated, but put simply, this is for those who want a declarative base system with an imperative development process akin to what you might expect in a normal distribution (i.e. Arch Linux or Fedora).

Existing solutions for this in NixOS and/or nixpkgs include the "default" virtual machine (VM) approach and the fake FHS user environments (whether provided by `buildFHSUserEnv` or running with `steam-run`).

Using a virtual machine works well as a solution, except there are its own overheads induced by using one. In order to use its binaries, it would need to have access to the shared directory, which could be easily solved through VirtIO-FS as long as it isn't a complex directory structure. However, the more fundamental issue is the battery drain and unnecessary computational cycles caused by using a virtual machine in the first place, since it simulates an entire operating system.

`buildFHSUserEnv` and `steam-run` work fine for most problems, but as with the input-addressed or content-addressed derivations, they are always fixed, which means that either the closure size will explode to take care of all scenarios or it will only "just work". The more fundamental issue is that this is a "hack-ish" method of forcibly allowing non-NixOS compatible software to work on NixOS.

This does not aim to allow for __any__ binary from arbitrary distribution to work correctly, such as binary that attempts to change one of the read-only files in NixOS, say `/etc/static/fstab`, but this shouldn't be an issue for development.

This is __NOT__ Bedrock Linux, if anything, it is just allows for the sharing of binaries from multiple Linux Distributions, however it may be possible to simulate similar behavior as it may be possible to share the final unioned view of the filesystem through VirtIO-FS to feed to the VMs such that they can modify the base root filesystem, thereby allowing a "mixture" of init systems and binaries from different distributions.

# Concept

Using a mixture of concepts and recent kernel features, namely union filesystems (mergerfs) [1], chroot[ jails], and VirtIO-FS, it is possible to create an alternative "root" filesystem view which should streamline the development process far smoother compared existing solutions in NixOS.

By isolating each distribution to its own separate "root" folder, this effectively allows for a "clean" view of the base root filesystem untouched by any other distributions. No unnecessary state is fed in-between each of the distribution's root filesystems.

A high-level overview of the processes is as follows:

1. Determine a folder to use as a root filesystem for the incoming Linux Distribution

2. Create a Virtual Machine that has access to said folder and mount as the root folder for the installation process through VirtIO-FS

3. Use `mergerfs` to create a unioned root filesystem combining the base root folder and the installed distribution's root folder

Packages can be installed by either booting back the VM with the same root filesystem or installing within a `chroot` of the unioned filesystem (i.e., if it modifies systemd you don't want the changes to propagate to your base system, though correct isolation should prevent this don't do it in the first place).

In conclusion, this is just a method which takes allows the end-user to take advantage of NixOS where it shines and fallback to a more "user-friendly" approach when necessary.

[1] https://github.com/trapexit/mergerfs
