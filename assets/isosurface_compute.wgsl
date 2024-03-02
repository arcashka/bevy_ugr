struct PolygonizationInfo {
    grid_size: vec3<f32>,
    grid_origin: vec3<f32>, // bottom left corner
    sphere_center: vec3<f32>,
    sphere_radius: f32,
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

struct CellInfo {
    vbo_index: u32,
    intersections_bitmask: u32,
}

@group(0) @binding(0) var<uniform> polygonization_info: PolygonizationInfo;
@group(0) @binding(1) var<storage, read_write> vbo: array<f32>;
@group(0) @binding(2) var<storage, read_write> ibo: array<u32>;
@group(0) @binding(3) var<storage, read_write> cells: array<CellInfo>;
@group(0) @binding(4) var<storage, read_write> atomics: array<atomic<u32>, 2>;
@group(0) @binding(5) var<storage, read_write> indices: Indices;
@group(0) @binding(6) var<storage, read_write> indirect: DrawIndexedIndirect;

fn flat_invocation_id(invocation_id: vec3<u32>, invocations_number: vec3<u32>) -> u32 {
    return invocation_id.x + invocation_id.y * invocations_number.x + invocation_id.z * invocations_number.x * invocations_number.y;
}

// because vec3f has 16 bytes alighnment
fn set_vertex(index: u32, value: vec3<f32>, normal: vec3<f32>) {
    let offset = index * 6;
    vbo[offset] = value.x;
    vbo[offset + 1] = value.y;
    vbo[offset + 2] = value.z;
    vbo[offset + 3] = normal.x;
    vbo[offset + 4] = normal.y;
    vbo[offset + 5] = normal.z;
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
    return distance(x, polygonization_info.sphere_center) - polygonization_info.sphere_radius;
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

fn write_quad_to_ibo(index: u32, point0: u32, point1: u32, point2: u32, point3: u32) {
    ibo[index * 6] = point0;
    ibo[index * 6 + 1] = point1;
    ibo[index * 6 + 2] = point2;
    ibo[index * 6 + 3] = point1;
    ibo[index * 6 + 4] = point3;
    ibo[index * 6 + 5] = point2;
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
    let vortex_origin = polygonization_info.grid_origin + (vec3<f32>(invocation_id) * vortex_size);
    var vertices = cube_vertices(vortex_size, vortex_origin);
    var sdfs = sdfs(vertices);

    var sum = vec3<f32>(0.0, 0.0, 0.0);
    var intersections_count: u32 = 0;
    var intersections_bitmask: u32 = 0;
    for (var i: u32; i < 12; i++) {
        let edge = edges[i];
        let p0_index = edge[0];
        let p1_index = edge[1];
        let sdf0 = sdfs[p0_index];
        let sdf1 = sdfs[p1_index];
        if ((sdf0 > 0.0) != (sdf1 > 0.0)) {
            intersections_bitmask |= edge_bitmask(i);
            sum += get_intersection(vertices[p0_index], vertices[p1_index], sdf0, sdf1);
            intersections_count += 1u;
        }
    }
    var vbo_index: u32 = 0;
    if intersections_count > 0 {
        let point = sum / f32(intersections_count);
        let normal = normal(sdfs);
        vbo_index = atomicAdd(&atomics[0], 1u);
        set_vertex(vbo_index, point, normal);
    }
    let flat_index = flat_invocation_id(invocation_id, invocations_number);
    cells[flat_index] = CellInfo(vbo_index, intersections_bitmask);
}

@compute @workgroup_size(8, 8, 8)
fn connect_vertices(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let invocations_number = num_workgroups * vec3<u32>(8, 8, 8);
    let cells_index_point0 = flat_invocation_id(invocation_id, invocations_number);
    let cell1 = cells[cells_index_point0];
    if ((cell1.intersections_bitmask & edge_bitmask(0u)) != 0u) {
        if (invocation_id.y != 0 && invocation_id.z != 0) {
            let vbo_index_point0 = cell1.vbo_index;
            let vbo_index_point1 = cells[flat_invocation_id(invocation_id - vec3<u32>(0, 1, 0), invocations_number)].vbo_index;
            let vbo_index_point2 = cells[flat_invocation_id(invocation_id - vec3<u32>(0, 0, 1), invocations_number)].vbo_index;
            let vbo_index_point3 = cells[flat_invocation_id(invocation_id - vec3<u32>(0, 1, 1), invocations_number)].vbo_index;
            let quad_index = atomicAdd(&atomics[1], 1u);
            write_quad_to_ibo(quad_index, vbo_index_point0, vbo_index_point1, vbo_index_point2, vbo_index_point3);
        }
    }
    if ((cell1.intersections_bitmask & edge_bitmask(1u)) != 0u) {
        if (invocation_id.x != 0 && invocation_id.z != 0) {
            let vbo_index_point0 = cell1.vbo_index;
            let vbo_index_point1 = cells[flat_invocation_id(invocation_id - vec3<u32>(1, 0, 0), invocations_number)].vbo_index;
            let vbo_index_point2 = cells[flat_invocation_id(invocation_id - vec3<u32>(0, 0, 1), invocations_number)].vbo_index;
            let vbo_index_point3 = cells[flat_invocation_id(invocation_id - vec3<u32>(1, 0, 1), invocations_number)].vbo_index;
            let quad_index = atomicAdd(&atomics[1], 1u);
            write_quad_to_ibo(quad_index, vbo_index_point0, vbo_index_point1, vbo_index_point2, vbo_index_point3);
        }
    }
    if ((cell1.intersections_bitmask & edge_bitmask(2u)) != 0u) {
        if (invocation_id.x != 0 && invocation_id.y != 0) {
            let vbo_index_point0 = cell1.vbo_index;
            let vbo_index_point1 = cells[flat_invocation_id(invocation_id - vec3<u32>(1, 0, 0), invocations_number)].vbo_index;
            let vbo_index_point2 = cells[flat_invocation_id(invocation_id - vec3<u32>(0, 1, 0), invocations_number)].vbo_index;
            let vbo_index_point3 = cells[flat_invocation_id(invocation_id - vec3<u32>(1, 1, 0), invocations_number)].vbo_index;
            let quad_index = atomicAdd(&atomics[1], 1u);
            write_quad_to_ibo(quad_index, vbo_index_point0, vbo_index_point1, vbo_index_point2, vbo_index_point3);
        }
    }
}

@compute @workgroup_size(1, 1, 1)
fn prepare_indirect_buffer() {
    indirect.index_count = atomics[1] * 6u;
    indirect.instance_count = indices.count;
    indirect.first_index = 0u;
    indirect.vertex_offset = 0i;
    indirect.first_instance = indices.start;
}

