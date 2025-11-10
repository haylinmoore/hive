{ lib }:

{
  mkServiceDerivation =
    {
      name,

      # What to run
      command ? null,
      package ? null,

      # Environment
      environment ? { },
      workingDirectory ? null,

      # User/Group
      user ? null,
      group ? null,

      # Lifecycle hooks
      preStart ? null,
      postStart ? null,
      preStop ? null,
      postStop ? null,

      # Dependencies
      after ? [ ],
      wants ? [ ],
      requires ? [ ],

      # Metadata for wrappers like wrapVirtualHost
      meta ? { },
    }:
    {
      inherit
        name
        command
        package
        environment
        workingDirectory
        ;
      inherit user group;
      inherit
        preStart
        postStart
        preStop
        postStop
        ;
      inherit after wants requires;
      inherit meta;
    };
}
