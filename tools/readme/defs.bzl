load("@aspect_bazel_lib//lib:write_source_files.bzl", "write_source_files")

def readme_from_registry_metadata(name, template, metadata, categorization_rules):
    """Generate README from module metadata and write it to source."""

    gen_name = name + "_gen"
    out_name = name + ".generated.md"

    native.genrule(
        name = gen_name,
        srcs = [
            template,
            metadata,
            categorization_rules,
        ],
        tools = ["//:tools/readme/generate_readme.py"],
        outs = [out_name],
        cmd = "python3 $(location //:tools/readme/generate_readme.py) " +
              "--template $(location %s) " % template +
              "--rules $(location %s) " % categorization_rules +
              "--out $@ " +
              "$(locations %s)" % metadata,
    )

    write_source_files(
        name = name + ".write",
        files = {"README.md": ":" + gen_name},
        diff_test = True,
    )
