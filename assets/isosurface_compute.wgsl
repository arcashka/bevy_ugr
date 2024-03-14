struct PolygonizationInfo {
    grid_size: vec3<f32>,
    grid_location: vec3<f32>,
}

struct DrawIndexedIndirect {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    vertex_offset: i32,
    first_instance: u32,
}

struct Indices {
    start: u32,
    count: u32,
}

// The grid used
//       6 ---- 7
//      /|     /|
//     4----- 5 |
//     | 2----|-3
//     |/     |/
//     0 ---- 1
//
struct VertexInfo {
    flat_cell_index: u32,
    intersections_bitmask: u32,
}

struct Atomics {
    vertex_count: atomic<u32>,
    quad_count: atomic<u32>,
}

@group(0) @binding(0) var<uniform> polygonization_info: PolygonizationInfo;
@group(0) @binding(1) var<storage, read_write> vbo: array<f32>;
@group(0) @binding(2) var<storage, read_write> ibo: array<u32>;
@group(0) @binding(3) var<storage, read_write> vertices: array<VertexInfo>;
@group(0) @binding(4) var<storage, read_write> atomics: Atomics;
@group(0) @binding(5) var<storage, read_write> indices: Indices;
@group(0) @binding(6) var<storage, read_write> indirect: DrawIndexedIndirect;

fn flat_invocation_id(invocation_id: vec3<u32>, invocations_number: vec3<u32>) -> u32 {
    return invocation_id.x + invocation_id.y * invocations_number.x + invocation_id.z * invocations_number.x * invocations_number.y;
}

// because vec3f has 16 bytes alighnment
fn set_vbo_data(index: u32, value: vec3<f32>, normal: vec3<f32>) {
    let offset = index * 6;
    vbo[offset] = value.x;
    vbo[offset + 1] = value.y;
    vbo[offset + 2] = value.z;
    vbo[offset + 3] = normal.x;
    vbo[offset + 4] = normal.y;
    vbo[offset + 5] = normal.z;
}

fn get_vbo_data(index: u32) -> array<vec3<f32>, 2>{
    let offset = index * 6;
    return array<vec3<f32>, 2>(
        vec3<f32>(vbo[offset], vbo[offset + 1], vbo[offset + 2]),
        vec3<f32>(vbo[offset + 3], vbo[offset + 4], vbo[offset + 5]),
    );
}

fn cube_vertices(vortex_size: vec3<f32>, vortex_origin: vec3<f32>) -> array<vec3<f32>, 8>{
    return array<vec3<f32>, 8>(
        vortex_size * vec3<f32>(0.0, 0.0, 0.0) + vortex_origin,
        vortex_size * vec3<f32>(1.0, 0.0, 0.0) + vortex_origin,
        vortex_size * vec3<f32>(0.0, 1.0, 0.0) + vortex_origin,
        vortex_size * vec3<f32>(1.0, 1.0, 0.0) + vortex_origin,
        vortex_size * vec3<f32>(0.0, 0.0, 1.0) + vortex_origin,
        vortex_size * vec3<f32>(1.0, 0.0, 1.0) + vortex_origin,
        vortex_size * vec3<f32>(0.0, 1.0, 1.0) + vortex_origin,
        vortex_size * vec3<f32>(1.0, 1.0, 1.0) + vortex_origin,
    );
}

fn sdf(x: vec3<f32>) -> f32 {
    return distance(x, vec3<f32>(0.0, 0.0, 0.0)) - 3.0;
}

fn sdfs(vertices: array<vec3<f32>, 8>) -> array<f32, 8> {
    return array<f32, 8>(
        sdf(vertices[0]),
        sdf(vertices[1]),
        sdf(vertices[2]),
        sdf(vertices[3]),
        sdf(vertices[4]),
        sdf(vertices[5]),
        sdf(vertices[6]),
        sdf(vertices[7]),
    );
}

fn normal(sdfs: array<f32, 8>) -> vec3<f32> {
    let dx = (sdfs[1] - sdfs[0]) + (sdfs[3] - sdfs[2]) + (sdfs[5] - sdfs[4]) + (sdfs[7] - sdfs[6]);
    let dy = (sdfs[2] - sdfs[0]) + (sdfs[3] - sdfs[1]) + (sdfs[6] - sdfs[4]) + (sdfs[7] - sdfs[5]);
    let dz = (sdfs[4] - sdfs[0]) + (sdfs[5] - sdfs[1]) + (sdfs[6] - sdfs[2]) + (sdfs[7] - sdfs[3]);
    return normalize(vec3<f32>(dx, dy, dz));
}

fn edge_bitmask(index: u32) -> u32 {
    return 1u << index;
}

// linear
fn get_intersection(p0: vec3<f32>, p1: vec3<f32>, sdf0: f32, sdf1: f32) -> vec3<f32> {
    let ratio = sdf0 / (sdf0 - sdf1);
    return (1.0 - ratio) * p0 + ratio * p1;
}

fn write_quad_to_ibo(index: u32, point0: u32, point1: u32, point2: u32, point3: u32, cw: bool) {
    if (cw) {
        ibo[index * 6] = point0;
        ibo[index * 6 + 1] = point1;
        ibo[index * 6 + 2] = point2;
        ibo[index * 6 + 3] = point1;
        ibo[index * 6 + 4] = point3;
        ibo[index * 6 + 5] = point2;
    } else {
        ibo[index * 6] = point2;
        ibo[index * 6 + 1] = point1;
        ibo[index * 6 + 2] = point0;
        ibo[index * 6 + 3] = point2;
        ibo[index * 6 + 4] = point3;
        ibo[index * 6 + 5] = point1;
    }
}

@compute @workgroup_size(8, 8, 8)
fn find_vertices(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    var edges = array<vec2<u32>, 12>(
        vec2<u32>(0, 1),
        vec2<u32>(0, 2),
        vec2<u32>(0, 4),
        vec2<u32>(1, 3),
        vec2<u32>(1, 5),
        vec2<u32>(2, 3),
        vec2<u32>(2, 6),
        vec2<u32>(3, 7),
        vec2<u32>(4, 5),
        vec2<u32>(4, 6),
        vec2<u32>(5, 7),
        vec2<u32>(6, 7)
    );

    let invocations_number = num_workgroups * vec3<u32>(8, 8, 8);
    let vortex_size = polygonization_info.grid_size / vec3<f32>(invocations_number);
    let vortex_origin = (polygonization_info.grid_location - (polygonization_info.grid_size / 2.0)) + (vec3<f32>(invocation_id) * vortex_size);
    var local_vertices = cube_vertices(vortex_size, vortex_origin);
    var sdfs = sdfs(local_vertices);

    var sum = vec3<f32>(0.0, 0.0, 0.0);
    var intersections_count: u32 = 0;
    var intersections_bitmask: u32 = 0;
    for (var i: u32 = 0u; i < 12; i++) {
        let edge = edges[i];
        let p0_index = edge[0];
        let p1_index = edge[1];
        let sdf0 = sdfs[p0_index];
        let sdf1 = sdfs[p1_index];
        if ((sdf0 > 0.0) != (sdf1 > 0.0)) {
            sum += get_intersection(local_vertices[p0_index], local_vertices[p1_index], sdf0, sdf1);
            intersections_count += 1u;
            if (i < 3) {
                // here we care only about first 3 edges, so 0 - 1, 0 - 2, 0 - 4, see cube drawing above
                // the rest of u32 can be used to store directions for this 3 edges
                // which then can be used to figure out the order for connecting vertices
                intersections_bitmask |= edge_bitmask(i);
                if (sdf0 > 0.0) {
                    intersections_bitmask |= edge_bitmask(i + 3u);
                }
            }
        }
    }
    var index: u32 = 0;
    if intersections_count > 0 {
        let point = sum / f32(intersections_count);
        let normal = normal(sdfs);
        index = atomicAdd(&atomics.vertex_count, 1u);
        set_vbo_data(index, point, normal);
        let flat_index = flat_invocation_id(invocation_id, invocations_number);
        vertices[index] = VertexInfo(flat_index, intersections_bitmask);
    }
}

fn search_cell_index(target_flat_cell_index: u32) -> i32 {
    for (var i: u32 = 0u; i < atomics.vertex_count; i++) {
        if (vertices[i].flat_cell_index == target_flat_cell_index) {
            return i32(i);
        }
    }
    return -1;
}

@compute @workgroup_size(8, 8, 8)
fn connect_vertices(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let invocations_number = num_workgroups * vec3<u32>(8, 8, 8);
    let id = flat_invocation_id(invocation_id, invocations_number);
    let id0 = search_cell_index(id);
    if (id0 == -1) {
        return;
    }
    let cell1 = vertices[id0];
    if ((cell1.intersections_bitmask & edge_bitmask(0u)) != 0u) {
        if (invocation_id.y != 0 && invocation_id.z != 0) {
            let id1 = flat_invocation_id(invocation_id - vec3<u32>(0, 1, 0), invocations_number);
            let vbo_index_point1 = u32(search_cell_index(id1));
            let id2 = flat_invocation_id(invocation_id - vec3<u32>(0, 0, 1), invocations_number);
            let vbo_index_point2 = u32(search_cell_index(id2));
            let id3 = flat_invocation_id(invocation_id - vec3<u32>(0, 1, 1), invocations_number);
            let vbo_index_point3 = u32(search_cell_index(id3));
            let quad_index = atomicAdd(&atomics.quad_count, 1u);
            let order = (cell1.intersections_bitmask & edge_bitmask(3u)) == 0u;
            write_quad_to_ibo(quad_index, u32(id0), vbo_index_point1, vbo_index_point2, vbo_index_point3, order);
        }
    }
    if ((cell1.intersections_bitmask & edge_bitmask(1u)) != 0u) {
        if (invocation_id.x != 0 && invocation_id.z != 0) {
            let id1 = flat_invocation_id(invocation_id - vec3<u32>(1, 0, 0), invocations_number);
            let vbo_index_point1 = u32(search_cell_index(id1));
            let id2 = flat_invocation_id(invocation_id - vec3<u32>(0, 0, 1), invocations_number);
            let vbo_index_point2 = u32(search_cell_index(id2));
            let id3 = flat_invocation_id(invocation_id - vec3<u32>(1, 0, 1), invocations_number);
            let vbo_index_point3 = u32(search_cell_index(id3));
            let quad_index = atomicAdd(&atomics.quad_count, 1u);
            let order = (cell1.intersections_bitmask & edge_bitmask(4u)) != 0u;
            write_quad_to_ibo(quad_index, u32(id0), vbo_index_point1, vbo_index_point2, vbo_index_point3, order);
        }
    }
    if ((cell1.intersections_bitmask & edge_bitmask(2u)) != 0u) {
        if (invocation_id.x != 0 && invocation_id.y != 0) {
            let id1 = flat_invocation_id(invocation_id - vec3<u32>(1, 0, 0), invocations_number);
            let vbo_index_point1 = u32(search_cell_index(id1));
            let id2 = flat_invocation_id(invocation_id - vec3<u32>(0, 1, 0), invocations_number);
            let vbo_index_point2 = u32(search_cell_index(id2));
            let id3 = flat_invocation_id(invocation_id - vec3<u32>(1, 1, 0), invocations_number);
            let vbo_index_point3 = u32(search_cell_index(id3));
            let quad_index = atomicAdd(&atomics.quad_count, 1u);
            let order = (cell1.intersections_bitmask & edge_bitmask(5u)) == 0u;
            write_quad_to_ibo(quad_index, u32(id0), vbo_index_point1, vbo_index_point2, vbo_index_point3, order);
        }
    }
}

@compute @workgroup_size(1, 1, 1)
fn prepare_indirect_buffer() {
    indirect.index_count = atomics.quad_count * 6u;
    indirect.instance_count = indices.count;
    indirect.first_index = 0u;
    indirect.vertex_offset = 0i;
    indirect.first_instance = indices.start;
}

