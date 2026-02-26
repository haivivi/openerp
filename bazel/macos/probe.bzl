"""macOS probe — test macOS platform transition and collect build artifacts."""

load(":transition.bzl", "macos_arm64_transition")

def _macos_probe_impl(ctx):
    """Collect all files from transitioned deps into a directory."""
    out = ctx.actions.declare_directory(ctx.label.name + "_out")

    # Gather all dep files (forces them to be built)
    all_files = []
    for dep in ctx.attr.deps:
        if DefaultInfo in dep:
            all_files.extend(dep[DefaultInfo].files.to_list())

    # Copy them into the output dir
    cmds = ["mkdir -p {out}".format(out = out.path)]
    for f in all_files:
        cmds.append("cp {src} {out}/$(basename {src})".format(src = f.path, out = out.path))
    cmds.append("echo 'Probe collected {n} files'".format(n = len(all_files)))

    ctx.actions.run_shell(
        outputs = [out],
        inputs = all_files,
        command = "\n".join(cmds),
        mnemonic = "MacosProbe",
        progress_message = "Probing macOS transition output",
        use_default_shell_env = True,
    )

    return [DefaultInfo(files = depset([out]))]

macos_probe = rule(
    implementation = _macos_probe_impl,
    attrs = {
        "deps": attr.label_list(
            cfg = macos_arm64_transition,
        ),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
