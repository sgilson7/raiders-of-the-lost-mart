#version 300 es
// One shape: the mesh vertex, placed by the instance's three columns and
// translation (iA, iB, iC; translation in .w), coloured by iD (rgb, glow).
in vec3 aPos;
in vec3 aNormal;
// white on every mesh but the ground, whose vertices carry its colours
in vec3 aTint;
in vec4 iA;
in vec4 iB;
in vec4 iC;
in vec4 iD;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
out vec4 vColor;
// a facade's position on its panel, in metres: along the face, up it
out vec2 vFacade;
void main() {
  vec3 w = iA.xyz * aPos.x + iB.xyz * aPos.y + iC.xyz * aPos.z + vec3(iA.w, iB.w, iC.w);
  // the columns carry a non-uniform scale, so normals take the inverse
  // scale: divide each axis by its column's squared length
  vec3 n = iA.xyz * (aNormal.x / max(dot(iA.xyz, iA.xyz), 1e-8))
         + iB.xyz * (aNormal.y / max(dot(iB.xyz, iB.xyz), 1e-8))
         + iC.xyz * (aNormal.z / max(dot(iC.xyz, iC.xyz), 1e-8));
  vNormal = n;
  vWorld = w;
  vColor = vec4(iD.rgb * aTint, iD.a);
  vFacade = vec2((aPos.z + 0.5) * length(iC.xyz), (aPos.x + 0.5) * length(iA.xyz));
  gl_Position = uVP * vec4(w, 1.0);
}
