use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

// Class and member numbers come from the format spec. The values, type IDs and hierarchy are synthetic.
pub fn skeleton(lods: &[&[(i32, i32)]], rotation: [f32; 4], skeleton_class: &str) -> Vec<u8> {
    const FLOAT: u32 = 1;
    const INT: u32 = 2;
    const FLAGS: u32 = 3;
    const VEC: u32 = 4;
    const QUAT: u32 = 5;
    const LOCAL: u32 = 6;
    const MATRIX: u32 = 7;
    const BONE: u32 = 8;
    const BONES: u32 = 9;
    const LOD: u32 = 10;
    const LODS: u32 = 11;
    const RESOURCE_FILE: u32 = 12;
    const SKELETON: u32 = 13;
    const RESOURCE: u32 = 14;

    let mut fixture = MetaBuilder::new();
    for block in [
        TypeBlock::primitive(FLOAT, "FLOAT", 4, 4),
        TypeBlock::primitive(INT, "SLONG", 4, 4),
        TypeBlock::enumeration(FLAGS, "ETestFlags", 4),
        TypeBlock::class(
            VEC,
            "Vector3f",
            1,
            5,
            None,
            &[("1", FLOAT), ("2", FLOAT), ("3", FLOAT)],
        ),
        TypeBlock::class(
            QUAT,
            "Quaternion4f",
            1,
            5,
            None,
            &[("1", FLOAT), ("2", FLOAT), ("3", FLOAT), ("4", FLOAT)],
        ),
        TypeBlock::class(LOCAL, "QuatVect", 1, 5, None, &[("1", QUAT), ("2", VEC)]),
        TypeBlock::class(
            MATRIX,
            "Matrix34f",
            1,
            5,
            None,
            &[
                ("1", FLOAT),
                ("2", FLOAT),
                ("3", FLOAT),
                ("4", FLOAT),
                ("5", FLOAT),
                ("6", FLOAT),
                ("7", FLOAT),
                ("8", FLOAT),
                ("9", FLOAT),
                ("10", FLOAT),
                ("11", FLOAT),
                ("12", FLOAT),
            ],
        ),
        TypeBlock::class(
            BONE,
            "SkeletonBone",
            1,
            5,
            None,
            &[
                ("1", INT),
                ("2", INT),
                ("3", FLOAT),
                ("4", LOCAL),
                ("5", MATRIX),
                ("6", FLOAT),
                ("7", VEC),
                ("8", VEC),
                ("9", FLAGS),
            ],
        ),
        TypeBlock::short(BONES, "CStaticArray<SkeletonBone>", 0, 6, BONE),
        TypeBlock::class(
            LOD,
            "CSkeletonLOD",
            1,
            5,
            None,
            &[("1", FLOAT), ("4", FLAGS), ("2", BONES)],
        ),
        TypeBlock::short(LODS, "CStaticArray<CSkeletonLOD>", 0, 6, LOD),
        TypeBlock::class(RESOURCE_FILE, "CResourceFile", 1, 5, None, &[]),
        TypeBlock::class(RESOURCE, "CResource", 1, 5, None, &[("1", INT)]),
        TypeBlock::class(
            SKELETON,
            skeleton_class,
            1,
            5,
            Some(RESOURCE),
            &[("1", LODS)],
        ),
    ] {
        fixture.type_block(block);
    }

    // The base-class member also uses "1". The parser must select the member owned by CSkeleton.
    let mut data = ObjectBytes::new()
        .i32(123)
        .array_header(b"STAR", lods.len() as u32);
    for (index, bones) in lods.iter().enumerate() {
        data = data
            .f32(10.0 + 20.0 * index as f32)
            .u32(u32::MAX)
            .array_header(b"STAR", bones.len() as u32);
        for &(id, parent) in *bones {
            data = data.i32(id).i32(parent).f32(0.25);
            for field in rotation.into_iter().chain([
                1.0, 2.0, 3.0, 0.0, -1.0, 0.0, -5.0, 1.0, 0.0, 0.0, -6.0, 0.0, 0.0, 1.0, -7.0,
                3.75, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0,
            ]) {
                data = data.f32(field);
            }
            data = data.u32(5);
        }
    }
    fixture.object(0, RESOURCE_FILE, vec![]);
    fixture.object(1, SKELETON, data.finish());
    fixture.build().unwrap()
}
