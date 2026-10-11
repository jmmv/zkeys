# Instructions to prepare a new release

1.  Start a release change based on the latest `main` branch.

    ```sh
    jj git fetch
    jj new main
    ```

1.  Run the version update helper with the new release number:

    ```sh
    admin/update-version.sh X.Y.Z
    ```

    The script does the following:

    *   `NEWS.md`: Sets next version number, adds date, and cleans up notes.
    *   `README.md`: Updates latest version number and release date.
    *   `*/Cargo.toml`: Updates version number.
    *   `Cargo.lock`: Reflects the new version number.

1.  Submit the change for review and merge it after all checks pass.

    ```sh
    jj b s release
    jj git push
    ```

1.  Tag the merged commit as `zkeys-X.Y.Z` and push the tag.  The version in
    the tag must match the version in `Cargo.toml`.

    ```sh
    jj git fetch
    jj tag set zkeys-X.Y.Z -r main@origin
    jj git push --tag zkeys-X.Y.Z
    ```

1.  Wait for the release workflow to finish.  It runs the distribution checks
    for all supported platforms before publishing the GitHub release and its
    source archive.

1.  Go to the *Releases* page in GitHub and make sure the new release looks
    good (the notes are correct and the archive is ready for download).

1.  Push the new crate out.  This is the last step because it's not reversible:

    ```sh
    cargo publish
    ```
