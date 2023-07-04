const std = @import("std");

pub const POptions = struct {
    types: []PType,
    entrypoint: []const u8,
    packages: [][]const u8,
};

const PType = enum {
    Unknown,
    Python,
    Bash,
};

pub fn detectType(allocator: std.mem.Allocator, path: []const u8) ![]PType {
    var types = std.ArrayList(PType).init(allocator);
    defer types.deinit();

    var dir = try std.fs.openIterableDirAbsolute(path, .{});
    defer dir.close();

    var walker = try dir.walk(allocator);
    defer walker.deinit();

    while (true) {
        var d = try walker.next();
        if (d == null) {
            break;
        }
        std.debug.print("{s}\n", .{d.?.path});
    }

    return try types.toOwnedSlice();
}

test "walk_dir" {
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();
    const allocator = arena.allocator();

    var types = try detectType(allocator, "/home/tchaudhr/Workspace");
    _ = types;
}
