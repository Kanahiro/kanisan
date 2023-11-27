import { serve } from '@hono/node-server';
import { Hono } from 'hono';
import { Client } from 'pg';

import { generateMvt } from '../wkb2mvt/index';

// z/x/y -> NOT lon/lat bbox BUT web mercator bbox
function zxy2bbox(z: number, x: number, y: number): Float64Array {
    const n = 2.0 ** z;
    const lon_deg = (x / n) * 360.0 - 180.0;
    const lat_rad = Math.atan(Math.sinh(Math.PI * (1 - (2 * y) / n)));
    const lat_deg = (lat_rad * 180.0) / Math.PI;
    const lon_deg2 = ((x + 1) / n) * 360.0 - 180.0;
    const lat_rad2 = Math.atan(Math.sinh(Math.PI * (1 - (2 * (y + 1)) / n)));
    const lat_deg2 = (lat_rad2 * 180.0) / Math.PI;
    return new Float64Array([lon_deg, lat_deg2, lon_deg2, lat_deg]);
}

const client = new Client({
    connectionString: 'postgres://docker:docker@localhost:5432/postgres',
});
client.connect();

const app = new Hono();
app.get('/:table_name/:z/:x/:y', async (c) => {
    const table_name = c.req.param('table_name');
    const z = c.req.param('z');
    const x = c.req.param('x');
    const y = c.req.param('y');

    const bbox = zxy2bbox(parseInt(z), parseInt(x), parseInt(y));
    const result = await client.query(
        `select * from ${table_name} where ST_Intersects(wkb_geometry, ST_MakeEnvelope(${bbox[0]}, ${bbox[1]}, ${bbox[2]}, ${bbox[3]}, 4326))`,
    );

    const features = result.rows.map((r) => {
        const { wkb_geometry, ...rest } = r;
        return {
            wkb_buf: Buffer.from(wkb_geometry, 'hex'),
            properties: {},
        };
    });
    const data = {
        layer_name: 'test',
        features,
    };
    const mvt = await generateMvt(data, 4096, bbox);

    return c.body(mvt, 200, {
        'Content-Type': 'application/vnd.mapbox-vector-tile',
    });
});

serve(app);
