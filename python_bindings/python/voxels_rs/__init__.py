from .voxels_rs import *

try:
  from . import voxels_rs as _bin
  __doc__ = _bin.__doc__
except ImportError:
  pass

__all__ = [
  "Voxel"
]