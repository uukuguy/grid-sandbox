from Bio import PDB
import math

pdb_file = '/Users/sujiangwen/sandbox/LLM/speechless.ai/Autonomous-Agents/octo-sandbox/crates/octo-eval/datasets/gaia_files/7dd30055-0198-452e-8c25-f73dbe27dcb8.pdb'

parser = PDB.PDBParser(QUIET=True)
structure = parser.get_structure('5wb7', pdb_file)

# Get all atoms in order as they appear
atoms = list(structure.get_atoms())

atom1 = atoms[0]
atom2 = atoms[1]

print(f"Atom 1: {atom1.get_name()} in residue {atom1.get_parent().get_resname()} chain {atom1.get_parent().get_parent().get_id()}")
print(f"Atom 1 coords: {atom1.get_vector()}")
print(f"Atom 2: {atom2.get_name()} in residue {atom2.get_parent().get_resname()} chain {atom2.get_parent().get_parent().get_id()}")
print(f"Atom 2 coords: {atom2.get_vector()}")

diff = atom1.get_vector() - atom2.get_vector()
distance = diff.norm()

print(f"\nDistance between atom 1 and atom 2: {distance} Angstroms")
print(f"Rounded to nearest picometer (0.01 Angstroms): {round(distance, 2)} Angstroms")
