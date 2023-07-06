const std = @import("std");
const strs = @import("./string_utils.zig");
const testing = std.testing;

pub const POptions = struct {
    types: []PType,
    entrypoints: []Entrypoint,
    packages: [][]const u8,
};

const Entrypoint = struct {
    path: []const u8,
    interpreter: []const u8,
};

const PType = enum {
    Unknown,
    Python,
    Bash,
};

pub fn walkProject(allocator: std.mem.Allocator, path: []const u8) ![]POptions {
    var pops = std.ArrayList(POptions).init(allocator);
    defer pops.deinit();

    var dir = try std.fs.openIterableDirAbsolute(path, .{});
    defer dir.close();

    var walker = try dir.walk(allocator);
    defer walker.deinit();

    const kinds = std.fs.IterableDir.Entry.Kind;

    while (true) {
        var d = try walker.next();
        if (d == null) {
            break;
        }
        if (isHiddenPath(d.?.path)) {
            continue;
        }
        if (d.?.kind == kinds.file) {
            var interpreter = try getShebang(allocator, try std.fs.path.join(allocator, &[_][]const u8{ path, d.?.path }));
            if (interpreter != null) {
                std.debug.print("Found a Shebang Script!:{s}: {!s}\n", .{ d.?.path, interpreter.? });
            }
        }
    }

    return try pops.toOwnedSlice();
}

fn isHiddenPath(path: []const u8) bool {
    if (path.len == 0) {
        return false;
    }
    if (path[0] == '.') {
        return true;
    }
    if (strs.contains(path, "/.")) {
        return true;
    }
    return false;
}

test "detect hidden paths" {
    const cases = [_][]const u8{
        ".git/abc",
        "foo/.git",
        "foo",
        "foo/bar",
        "foo/ar",
        "foo/.git/bar",
        "foo/.git/.git",
        "foo/wow/.k.sh",
    };

    const expected = [_]bool{
        true,
        true,
        false,
        false,
        false,
        true,
        true,
        true,
    };

    for (cases, 0..) |path, i| {
        const actual = isHiddenPath(path);
        try testing.expectEqual(actual, expected[i]);
    }
}

// Returns the associated interpreter. Returns null if the file is not a shebang script.
fn getShebang(allocator: std.mem.Allocator, path: []const u8) !?[]const u8 {
    var file = try std.fs.openFileAbsolute(path, .{});
    defer file.close();

    var line = std.ArrayList(u8).init(allocator);
    defer line.deinit();

    var writer = line.writer();

    var reader = file.reader();

    // Improve this to handle errors better
    if (reader.streamUntilDelimiter(writer, '\n', null)) {} else |_| {
        return null;
    }

    const slice = try line.toOwnedSlice();
    if (strs.startsWith(slice, "#!")) {
        return slice[2..];
    } else {
        return null;
    }
}

test "walk_dir" {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    var types = try walkProject(allocator, "/home/tchaudhr/Workspace");
    _ = types;
}
