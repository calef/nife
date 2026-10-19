# Many indexes per machine: the four rulings

An appendix of [the package client becomes a program](../0809-the-package-client-becomes-a-program.md),
milestone 809, holding the rulings item 7 (`jig add-repository <url>`) is built against. It is a
separate file because the block would otherwise pass §212 (a prose budget)'s cap.

calef, 2026-10-06 (UTC): *"A machine may write many indexes."* So a machine holds several, as apt's
`sources.list` does; details below.

§250 (an image names its distribution's package index) fixes only an image's one index, basalt's;
an index the owner adds is the machine's. §195 clause 2 scopes trust per source, and §196 clause 4
holds roots per source.

Its four details are all ruled:

1. How an added index is authenticated: ruled 2026-10-17 (UTC). calef: *"Just a URL."*
   `jig add-repository <url>` takes only a URL, and a refusal names the root the server needed.
   He first ruled that the index is checked against the roots the image pins for basalt (ISRG Root
   X1 today). §306 (the web's authority list is a package and a capability), decided 2026-10-18,
   replaces that set. An added index is trusted at the channel as a website is, by the list's
   authorities; basalt's own channel never depends on the list.
2. The same name in two indexes: ruled 2026-10-17 (UTC). calef chose *"A"*: a bare name offered by
   two indexes is refused as `Ambiguous`, as milestone 614 (two installed versions of one program, each runnable) does for versions, and an
   index-qualified name picks one. First-index-wins is refused as the dependency-confusion shape.
   The qualifier is `jig install --from <index> <name>`, with the index's URL exactly as added. The
   spelling is provisional (`Name: provisional` in
   [design/naming/README.md](../../naming/README.md), tracked by `script/names --unratified`). It is
   a flag because §219 reads a word containing `/` as a file, and `@` already means a version.
3. Provenance: ruled 2026-10-17 (UTC). Asked *"Record each row's vouching index by its URL?"*, calef
   answered *"Yes"*. Each installed-programs table row records the URL of the index that vouched
   for it, exactly as added, in place of `OWNER` alone, so a rollback can say what it is undoing.
4. An added index that says it moved: ruled 2026-10-17 (UTC). calef chose *"B"*: `jig` does not
   follow an added index's "moved to" field. It reports it and names the `jig add-repository <new
   url>` command. basalt's own index keeps following §250's field.
