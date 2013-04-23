package pt.adrz.clipx;

import java.awt.Point;

import javax.swing.JList;
import javax.swing.JTextField;


public class ClipList extends JList<String>{

	private static final long serialVersionUID = -2503993104430465274L;

	/**
	 * Field to search for String
	 */
	private ClipFilterField filterField;
	
	/**
	 * Model with String copied from clipboard
	 */
	private ClipFilterModel filterModel;
	
	private int FIELD_WIDTH = 20;
	
	public ClipList() {
		super();
		filterField = new ClipFilterField(FIELD_WIDTH, this);
		filterModel = new ClipFilterModel(filterField);
		this.setModel(filterModel);
	}
	
	
	
	public JTextField getFilterField() {
		return this.filterField;
	}
	
	
	public ClipFilterModel getModel() {
		return this.filterModel;
	}
	
	/**
	 * the original method returns -1 only if there is no element in the list
	 * I want -1 to be returned if no element is selected when a click is made
	 */
	@Override
	public int locationToIndex(Point location) {
		
		int index = super.locationToIndex(location);
		
		if ( index != -1 && !getCellBounds(index, index).contains(location))
			return -1;
		else
			return index;
	}
	
	
}
