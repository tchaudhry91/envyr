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

        if (d.?.kind == kinds.file) {
            if (try isExecutable(allocator, try std.fs.path.join(allocator, &[_][]const u8{ path, d.?.path }))) {
                std.debug.print("Found Executable!:{s}\n", .{d.?.path});
            }
        }
    }

    return try pops.toOwnedSlice();
}

// These are scripts that start with a shebang line.
fn isExecutable(allocator: std.mem.Allocator, path: []const u8) !bool {
    var file = try std.fs.openFileAbsolute(path, .{});
    defer file.close();

    var line = std.ArrayList(u8).init(allocator);
    defer line.deinit();

    var writer = line.writer();

    var reader = file.reader();
    try reader.streamUntilDelimiter(writer, '\n', null);

    if (strs.startsWith(try line.toOwnedSlice(), "#!")) {
        return true;
    } else {
        return false;
    }
}

test "test_executable" {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    try testing.expectEqual(true, try isExecutable(allocator, "/home/tchaudhr/test.sh"));
    try testing.expectEqual(false, try isExecutable(allocator, "/home/tchaudhr/test2.sh"));
}

test "walk_dir" {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    var types = try walkProject(allocator, "/home/tchaudhr/Workspace/mynt");
    _ = types;
}
